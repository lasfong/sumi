# KẾ HOẠCH KHẮC PHỤC TOÀN DIỆN END-TO-END (`PATCH R19.1 -> R19.3`) — DÀNH CHO CHẾ ĐỘ `/goal`

- **Mã Kế Hoạch:** `GOAL-PLAN-SUMI-R19-REMEDIATION-V1.0`
- **Dự Án:** Sumi Trading Workstation (`/Users/mizuhara/workspace/sumi`)
- **Tài Liệu Nguồn Bắt Buộc Đối Chiếu:**
  - Báo cáo lỗi gốc (`34` bugs): [`docs/testing/QA_BUG_REPORT.md`](./QA_BUG_REPORT.md)
  - Bộ kịch bản nghiệm thu (`65` test cases): [`docs/testing/SUMI_QA_QC_MASTER_TEST_PLAN.md`](./SUMI_QA_QC_MASTER_TEST_PLAN.md)
  - Ảnh chụp minh chứng lỗi trước khi sửa (`14` PNG): `docs/testing/qa_evidence/`
  - Yêu cầu gốc & sổ cái: [`docs/PROJECT_CHARTER.md`](../PROJECT_CHARTER.md), [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md)
- **Mục Tiêu Cuối Cùng (`Goal Exit Criteria`):**
  1. Khắc phục triệt để **`34/34` lỗi** (`4 P0`, `13 P1`, `14 P2`, `3 P3`).
  2. Đạt **`65/65` kịch bản `PASS`** (`100%`) trên `SUMI_QA_QC_MASTER_TEST_PLAN.md`.
  3. Toàn bộ `cargo test` (`267` test cũ + bộ test hồi quy mới cho đủ `34` lỗi) đạt **`100% PASS`**, không lỗi biên dịch/warning nghiêm trọng.
  4. Mọi benchmark `viewport_bench` (`--r12`, `--r17`, `--r18`, `--r19`) đạt `overall_pass: true` (`p99 < 4.0 ms`), đồng thời **`100K` + `2 AVWAP` `< 4.0 ms`**, **Watchlist `471` mã `< 1.5 ms`/frame**, **Replay `+1` kèm Watchlist `471` mã `< 16.0 ms` (`>= 60 FPS`)**.
  5. Chụp lại toàn bộ ảnh Metal GPU thực tế (`2560×1600` & `2880×2000`) và **soi pixel xác nhận bằng mắt (`read_file`)** cho mọi lỗi giao diện đã sửa.
  6. Đóng gói lại `/Users/mizuhara/workspace/sumi/dist/Sumi Workstation.app` (ký `codesign`, test mở từ `/tmp`), bảo toàn tuyệt đối `100%` dữ liệu gốc `CafeFData/*.zip` và `ARCHIVE/`, và xuất báo cáo tái kiểm thử [`docs/testing/QA_REAUDIT_REPORT_R19_PATCH.md`](./QA_REAUDIT_REPORT_R19_PATCH.md) với phán quyết **`GO FOR v1.0`**.

---

## 0. Quy Tắc Bất Biến Trong Suốt Quá Trình Chạy `/goal`

1. **Môi trường Rust:** Mọi lệnh `cargo` phải chạy kèm `export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"`.
2. **Bảo toàn tuyệt đối dữ liệu gốc (`TC-65`):**
   - TUYỆT ĐỐI KHÔNG sửa/xóa `/Users/mizuhara/workspace/work/CafeFData/*.zip`, `ARCHIVE/`, hay các tệp chứng cứ lịch sử đã đóng băng trong `docs/evidence/` (khi chạy `viewport_bench`, chạy trong thư mục tạm hoặc khôi phục sạch `docs/evidence/` nếu không có yêu cầu ghi đè).
   - KHÔNG đụng chạm ứng dụng TradingView Desktop trên máy (`DEC-006`).
3. **Kỷ luật Test-Driven Regression:** Mỗi Wave hoàn tất PHẢI đi kèm:
   - Test đơn vị / test tích hợp tái hiện đúng lỗi trước khi sửa và xác nhận PASS sau khi sửa (đặt trong `tests/` hoặc `#[cfg(test)]` của từng module).
   - Chạy `cargo test` xanh 100% trước khi chuyển sang Wave tiếp theo.
4. **Không cắt góc chất lượng:** Không tắt kiểm tra, không ẩn cảnh báo bằng cách bỏ tính năng, không dùng số liệu giả thay thế dữ liệu thật.

---

## 1. Lộ Trình Thực Thi `6` Wave Theo Đúng Thứ Tự Phụ Thuộc

```
Wave 1 (Lõi Tài Chính VN, Paper Trading, VN30F1M, T+2 & DOM Math)
   │
   ▼
Wave 2 (CafeF Data Loader, VNINDEX Thật, Khôi Phục Phiên, Cmd+K & Sàn VN)
   │
   ▼
Wave 3 (Nhân Quả Cutoff, Alerts, Strategy Lab, Backtest Analytics & PIT)
   │
   ▼
Wave 4 (IndicatorCache, Ổn Định Số Học NaN, AVWAP Memo & Drawings)
   │
   ▼
Wave 5 (TradingView UX Parity: Watchlist show_rows 60 FPS, RRG, Anti-Collision, Modals, 2x2, Toolbar & DOM UI)
   │
   ▼
Wave 6 (Đóng Gói .app Độc Lập, Benchmark, Soi Pixel Metal GPU 100%, Tái Kiểm Toán TC-01..TC-65 & Ký GO)
```

---

### WAVE 1 — Lõi Quy Chế Chứng Khoán VN, Paper Trading, `VN30F1M`, `T+2` FIFO & Toán Học DOM
- **Mục tiêu:** Đóng toàn bộ lỗi tài chính `P0` và sai lệch khớp lệnh/ký quỹ/kỷ luật thực hành.
- **Danh sách lỗi xử lý (`9` bugs):** `BUG-001` (`P0`), `BUG-002` (`P0`), `BUG-004` (`P0`), `BUG-008` (`P1`), `BUG-009` (`P1`), `BUG-017` (`P1`), `BUG-024` (phần toán học bậc giá `10.00` & risk equity), `BUG-030` (`P2`), `BUG-034` (phần lịch lễ sau `2030`, `stage_ladder_order`, nhãn `>80B`, làm tròn `SL Adh`).
- **Hạng mục kỹ thuật cụ thể:**
  1. **Mở rộng `AccountConfig` & `TradingSession`** ([`src/trading/orders.rs`](../../src/trading/orders.rs), [`src/trading/portfolio.rs`](../../src/trading/portfolio.rs), [`src/app.rs`](../../src/app.rs)):
     - Bổ sung `contract_multiplier` (mặc định `100.0` cho `Vn30Futures` đơn vị `kVND`, `1.0` cho cơ sở/US/Crypto), `initial_margin_pct` (`0.17` cho `Vn30Futures`), và `tax_sell_bps` (`10.0` bps = `0.10%` cho `HOSE`/`HNX`/`UPCOM`, `0.0` cho phái sinh/US/Crypto).
     - Áp dụng đồng bộ `contract_multiplier` vào mọi phép tính `realized_pnl`, `unrealized_pnl`, `notional`, phí giao dịch và kiểm tra ký quỹ ban đầu; trừ `tax_sell_bps` vào mọi giao dịch bán cổ phiếu cơ sở VN để khớp 100% với `src/journal/planner.rs` (`BUG-001`).
  2. **Sửa lỗi nhân đôi đòn bẩy DOM `VN30F1M`** ([`src/trading/ladder.rs:1068-1074`](../../src/trading/ladder.rs#L1068-L1074)):
     - Loại bỏ việc chia lặp `initial_margin_pct` và `max_leverage = 1 / 0.17` hai lần. Kiểm chứng `100.000 kVND` tại `1.300` kVND (`22.100 kVND/HĐ`) chỉ cho phép tối đa **`4` hợp đồng** (`BUG-002`).
     - Không chặn đáy vốn bằng `initial_cash` khi tài khoản đang lỗ (dùng `equity` thực tế tại `src/app.rs:4200-4204`, `BUG-024`).
  3. **Cưỡng chế `T+2` FIFO ngay trong lõi `TradingSession`** ([`src/trading/portfolio.rs:788-1125`](../../src/trading/portfolio.rs#L788-L1125), [`src/app.rs:3286-3293`](../../src/app.rs#L3286-L3293)):
     - Kiểm tra `enforce_vn_t2_lot_lock` trực tiếp bên trong `apply_submit_order_internal` (Market Sell + Limit/Stop Sell lúc khớp) và `apply_flatten_internal` (`BUG-004`).
     - Khi gọi `Flatten` với `200` cp đã về + `300` cp đang khóa (`T+2 Lock = ON`), tự động thanh lý đúng phần `200` cp `[SELLABLE]` hợp pháp và giữ nguyên `300` cp `[LOCKED T+2]` kèm thông báo rõ ràng.
     - Thống nhất chuẩn hóa mốc thời gian giữa `fifo_holding_summary` và `available_settled_long_qty_at` ([`src/journal/market_rules.rs:763-952`](../../src/journal/market_rules.rs#L763-L952)) cho cả `1D > 06:00 UTC`, `1W` base, và `UsEquities T+1` (`SPY 00:00 UTC` Thứ Hai -> Thứ Ba `T+1`, `BUG-017`).
  4. **Tách biệt quyền Bán khống (`allow_short`) khỏi `T+2 Lock`** ([`src/app.rs:2354-2357, 3842-3846`](../../src/app.rs), [`src/trading/portfolio.rs`](../../src/trading/portfolio.rs)):
     - Cấm bán khống trên `HOSE`/`HNX`/`UPCOM` (`!venue.allows_short()`) dù `T+2 Lock = ON` hay `OFF`; luôn cho phép Short trên `VN30F1M`/`US`/`Crypto` kể cả sau khi bấm `Apply` trong `[Settings]` (`BUG-008`).
     - Không gắn SL/TP chiều Short vào lệnh bán giảm vị thế Long trên `HOSE` (`stage_ladder_order`, `BUG-034`).
  5. **Sửa `DEF-005` Gap Phase A & Scale-In Mark-to-Market Margin** ([`src/trading/portfolio.rs:1149-1162, 1462-1568`](../../src/trading/portfolio.rs)):
     - Áp nhánh bi quan `SL trước TP` cho mọi lệnh chờ vừa khớp trên Gap mở cửa Phase A nếu thanh nến đó chạm cả hai biên (`BUG-009`).
     - Tính kiểm tra sức mua scale-in theo giá thị trường hiện tại (mark-to-market) + phí (`BUG-017`).
  6. **Sửa chuyển bậc giá `HOSE 10.00 kVND` trên DOM Ladder** ([`src/trading/ladder.rs:404-868`](../../src/trading/ladder.rs)):
     - So sánh dung sai theo bước giá cục bộ tại từng bậc, loại bỏ trùng 2 dòng `CEIL`, trùng 2 dòng `is_at_mark`, và đảm bảo Mua tại `10.00` khi `mark = 9.99` tạo `Buy Stop @ 10.00` (`BUG-024`).
  7. **Đồng bộ `Coaching`, `Debrief`, `TradeJournal`, `[Settings] Default Venue` & Lịch Lễ VN** ([`src/trading/coaching.rs`](../../src/trading/coaching.rs), [`src/journal/ledger.rs`](../../src/journal/ledger.rs), [`src/journal/market_rules.rs`](../../src/journal/market_rules.rs), [`src/app.rs`](../../src/app.rs)):
     - Không thổi phồng `TradeJournalStore::checklist.stop_and_lot_sized` hay chế `r_multiple` giả khi `UpdateBrackets` gắn `SL > Entry` sau vào lệnh (`BUG-030`).
     - Lọc `cutoff` nhất quán trên cả KPI tổng và bảng phân rã của `SessionDebriefReport::build`; lọc `PracticeDecisionNote` tương lai khi lưu phiên; đồng bộ `settings.enforce_vn_t2_lot_lock` khi khôi phục phiên; gọi `sync_trading_settlement_rules()` khi đổi `Default Venue` trong `[Settings]` (`BUG-030`).
     - Bổ sung ngày lễ VN giai đoạn `2016–2017` vào `VIETNAM_HOLIDAYS_V1` (phủ trọn `2016–2030` khớp `CafeFData`), sửa fallback Quốc khánh sau `2030` khi `02/09` là Thứ Sáu (`2033-09-02`), sửa hiển thị `"79.x% (< 80% Target)"` và khóa cắt nhãn `>80B` trong store (`BUG-021`, `BUG-034`).
- **Cổng kiểm chứng Wave 1 (`Gate W1`):**
  - Thêm bài test hồi quy tự động xác nhận: `VN30F1M` PnL `-4.19 pts -> -418.94 kVND`; DOM risk sizing = `4` HĐ; `FPT` bán trừ đủ `0.10%` thuế; `TradingSession` chặn Market Sell `T+1`; `HOSE` cấm Short khi `T+2=OFF` & `VN30F1M` Short được sau `Settings Apply`; Gap Phase A chốt `SL=98.0`; DOM `HOSE 10.00` không trùng `CEIL`/`Mark` & Mua `10.00 > 9.99` là `Stop`; Tết `2016-02-05` mở khóa `2016-02-17`.
  - `cargo test` đạt `100% PASS`.

---

### WAVE 2 — Toàn Vẹn Dữ Liệu `CafeFData`, Benchmark `VNINDEX` Thật, Khôi Phục Phiên, `Cmd+K` & Phân Loại Sàn VN
- **Mục tiêu:** Đảm bảo `471` mã CafeF và `VNINDEX`/`HNX-INDEX` thật luôn chính xác, bền vững qua khởi động lại, và không bao giờ bị ghi đè bởi dữ liệu giả lập.
- **Danh sách lỗi xử lý (`9` bugs):** `BUG-003` (`P0`), `BUG-005` (`P1`), `BUG-006` (`P1`), `BUG-007` (`P1`), `BUG-012` (`P1`), `BUG-013` (`P1`), `BUG-021` (`P2`), `BUG-025` (`P2`), `BUG-031` (`P2`).
- **Hạng mục kỹ thuật cụ thể:**
  1. **Sửa `CafeFSplitAware` Rule 5** ([`src/data/audit.rs:235-243`](../../src/data/audit.rs#L235-L243)):
     - Loại bỏ nhánh sai `None if delta_max_vnd > 1 ULP` không có trong `cafef_importer.py`, khôi phục **`23.550` phiên phẳng sau chia tách hợp lệ** trên `421/471` mã (`RCL`, `KSV`, `APS`, `API`, `C69`, `BVS`, `HNX-INDEX`...) trong khi vẫn cách ly sạch 100% lỗi `High = 0` ngày KRX `05/05/2025` và lỗi `HNX-INDEX High = 3` (`BUG-005`).
  2. **Nạp `VNINDEX` / `HNX-INDEX` thật tự động cho mọi benchmark** ([`src/data/presets.rs:385, 536-572`](../../src/data/presets.rs), [`src/app.rs:5404`](../../src/app.rs)):
     - Ưu tiên tìm nạp `data/raw/cafef/CafeF.INDEX.Upto06112026.csv` (hoặc tệp `CafeF.INDEX*.csv` mới nhất trong `data/raw/cafef/`), đồng thời tự gắn `VNINDEX` thật vào tập benchmark khi nạp `CafeF Adj/Raw` (`BUG-006`).
     - Không rơi ngầm về đường sóng sin giả lập khi có `VNINDEX` thật phủ cùng dải ngày; nếu thực sự không có tệp thật trên máy, gắn cờ `is_synthetic_benchmark = true` và hiển thị huy hiệu cảnh báo `[SYNTH]` rõ ràng trên HUD/Legend.
  3. **Sửa khôi phục phiên `CustomFile` khi khởi động lại** ([`src/app.rs:1572-1640`](../../src/app.rs)):
     - Lưu cấu hình `DataLoadOptions` (`InvalidRowPolicy::QuarantineRow`, `OhlcEnvelopePolicy::CafeFSplitAware`...) trong persisted state và sử dụng đúng cấu hình đó khi gọi `restore_persisted_session()`, đảm bảo đóng/mở lại app khôi phục nguyên vẹn `FPT (2702 nến)`, con trỏ Replay, nét vẽ và vị thế (`BUG-003`).
  4. **Sửa thứ hạng `Cmd+K` và chống xóa `CustomFile`** ([`src/app.rs:8023-8044, 8744-8752, 4871-4896`](../../src/app.rs)):
     - Khi đang nạp nguồn `CustomFile`, cộng điểm ưu tiên tuyệt đối (hoặc ẩn bản sao giả lập `VN30 Basket` trùng mã) để gõ `VCB`/`FPT`/`HPG`/`SSI` + `Enter` luôn chuyển trong chính bộ `471` mã thật mà không gọi `CustomSourceUpdate::Clear` (`BUG-007`).
  5. **Sửa chế độ tắt `CafeF Split Norm` (`TC-06`) & `TimestampUnit::Auto` & `QuarantineRow`** ([`src/app.rs:3789-3799`](../../src/app.rs), [`src/data/mod.rs:640-644`](../../src/data/mod.rs), [`src/data/calendar.rs:537-553`](../../src/data/calendar.rs), [`src/data/csv_loader.rs`](../../src/data/csv_loader.rs), [`src/data/json_loader.rs`](../../src/data/json_loader.rs), [`src/replay/controller.rs:504`](../../src/replay/controller.rs)):
     - Khi người dùng bỏ tick `CafeF Split Norm`, trả lỗi đỏ ngay khi gặp vi phạm biên OHLC (`OhlcEnvelopePolicy::RejectRow`) và **giữ nguyên biểu đồ hiện tại đang mở** (`BUG-012`).
     - Sửa `TimestampUnit::Auto`: chỉ coi là Milliseconds khi `abs(ts) >= 100_000_000_000` (`>= 10^11`), chấm dứt biến năm `2024` chẵn `000` thành `1970` (`BUG-013`).
     - Đồng bộ `QuarantineRow` cho cả CSV đơn mã, CSV đa mã, JSON loader; bỏ qua dòng trống đầu tệp khi dò delimiter; hạ trạng thái audit khi có trùng timestamp; chống trôi tổng RVOL & vòng lặp ngày lịch vô hạn; dùng `saturating_add(1)` ở `ReplayController::materialize_candles` (`BUG-031`).
  6. **Phân loại chính xác sàn `HOSE` / `HNX` / `UPCOM` & dữ liệu mẫu chuẩn lịch VN** ([`src/journal/market_rules.rs:116-128`](../../src/journal/market_rules.rs), [`src/data/audit.rs:548-566`](../../src/data/audit.rs), [`src/data/synthetic.rs`](../../src/data/synthetic.rs), [`src/data/presets.rs:303-310`](../../src/data/presets.rs)):
     - Mở rộng bảng ánh xạ sàn `HNX`/`UPCOM` (hoặc suy diễn theo phân bố bước giá/biên độ kết hợp bảng mã chính thức VN cho toàn bộ `471` mã CafeF) để `C69`, `KSV`, `NDN`, `BVS`, `API`, `APS`... nhận đúng sàn `HNX`/`UPCOM`, nâng tỷ lệ `Feeds Ready` từ `12/471` lên mức phản ánh đúng thực tế (`BUG-021`).
     - Điều chỉnh ngưỡng `EXTREME_RANGE` theo sàn (`HOSE` `>20%`, `HNX` `>28%`, `UPCOM` `>42%`) và theo khung thời gian (`1W` không dùng ngưỡng ngày).
     - Đồng bộ `timestamp` và `date_label` của `VNINDEX Synth` theo ngày giao dịch hợp lệ `VIETNAM_HOLIDAYS_V1` (`0` thanh cuối tuần/ngày lễ); dịch chuỗi `VN30F1M 1m` bỏ qua ngày lễ Quốc khánh `01–02/09/2026` và bật kiểm tra ngày lễ intraday trong `[Data Audit]` (`BUG-025`).
- **Cổng kiểm chứng Wave 2 (`Gate W2`):**
  - Nạp `CafeF Adj (471 mã)` vẫn `< 600 ms`; `RCL` giữ `~1.993` nến (không còn mất `798` nến); `BVS`/`API`/`APS` & `VNINDEX`/`HNX-INDEX` `min_low > 0`; `FPT` `Cmp:VNINDEX` dùng `VNINDEX` thật (`574.41 -> 1725.36`, `+200.37%`); `Cmd+K` `"VCB"` + Enter giữ nguyên `471` mã; `restore_persisted_session` giữ nguyên `FPT` + Replay cursor; bỏ tick `Split Norm` trả lỗi đỏ giữ nguyên biểu đồ; `1728000000` giải mã đúng `2024-10-04`.
  - `cargo test` đạt `100% PASS`.

---

### WAVE 3 — Nhân Quả Cutoff, Hệ Thống Cảnh Báo (`Alerts`), Strategy Lab & Analytics
- **Mục tiêu:** Đảm bảo tuyệt đối `DEF-001` đơn điệu trên mọi phân hệ và sửa toàn bộ lỗi toán học/logic trong `Alerts`, `Backtest`, `Tearsheet`, `VN30-PIT`.
- **Danh sách lỗi xử lý (`6` bugs):** `BUG-010` (`P1`), `BUG-011` (`P1`), `BUG-016` (`P1`), `BUG-028` (`P2`), `BUG-029` (`P2`), `BUG-034` (phần `rsi_overbought_cross`, `UniverseRemoval` peak, `export_markdown`, `eval_base_timestamp`).
- **Hạng mục kỹ thuật cụ thể:**
  1. **Thống nhất hàm cắt nhân quả đơn điệu (`Causal Cutoff`)** ([`src/backtest/engine.rs:780-839, 2542-2594`](../../src/backtest/engine.rs), [`src/scanner/proxy_bb.rs:268-278`](../../src/scanner/proxy_bb.rs)):
     - Đảm bảo số lượng thanh nến hiển thị/đánh giá là **hàm đơn điệu không giảm** khi `cutoff` tăng (`07:59:59 -> 08:00:00 UTC` VN và `20:59:59 -> 21:00:00 UTC` US không bao giờ tụt nến), tính đúng US EDT/EST và phân loại đúng sàn cho mã US ngoài whitelist (`GOOG`, `AMZN`, `META`, `BUG-010`).
  2. **Sửa 3 lỗi nhân quả trong `Alerts`** ([`src/alerts/engine.rs:984-1345`](../../src/alerts/engine.rs), [`src/alerts/mod.rs:250-255`](../../src/alerts/mod.rs)):
     - Bắt buộc `bar_ts > arm_ts` (chặt sau nến đặt lệnh) cho `RsiThreshold`, `MacdHistogramZeroCross`, `DrawingLine` để không nổ hồi tố trên `SPY 14:30 UTC` (`BUG-011`).
     - Không đánh giá `On Bar Close` trên thanh HTF `1W` chưa đóng cửa giữa tuần (`BUG-011`).
     - Luôn gắn `created_at_timestamp = Some(latest_bar_ts)` khi tạo cảnh báo ở chế độ Live để ẩn sạch khi tua lùi Replay về quá khứ (`BUG-028`).
     - Đổi `eval_base_timestamp` sang `Option<i64>` tránh trùng mốc `0` (`1970-01-01`, `BUG-034`).
  3. **Sửa Backtest, `OutlierImpact`, `Yearly`, `Breadth` & `VN30-PIT`** ([`src/backtest/analytics.rs:426-437, 669-671`](../../src/backtest/analytics.rs), [`src/backtest/engine.rs:427-475, 974-1597, 2503-2534`](../../src/backtest/engine.rs), [`src/backtest/walk_forward.rs:914`](../../src/backtest/walk_forward.rs), [`src/scanner/breadth.rs:197-205`](../../src/scanner/breadth.rs), [`src/app.rs:7561`](../../src/app.rs)):
     - Sửa mẫu số `expectancy_ex_top_3_winners` thành `valid_n - removed_winners_count` (`[+50, -100, -100, -100]` -> `-100.0`, `BUG-016`).
     - Chặn bán khống trái phép trên sàn cơ sở VN trong `Strategy Lab` / `Battle` / `Portfolio` khi `!venue.allows_short()` (`BUG-008`).
     - Sửa cắt trùng 2 lần trong `compute_market_breadth_and_rrg`; áp đúng bộ lọc `VN30 Only` trên `CustomFile` ở `run_active_portfolio_backtest`; đồng bộ mốc ngày `W1` PIT cho `UniverseRemoval` & loại bỏ đỉnh vốn ảo; thêm bảo vệ `price.is_finite() && price > 0.0` cho mọi lệnh thoát/đảo chiều; không báo `-100.0%` giả ở các năm sau phá sản (`BUG-029`, `BUG-034`).
     - Sửa metadata mặc định `technical.rsi_overbought_cross` từ `30.0` -> `70.0` và lọc ký tự `\r`/`\n` trong `export_markdown` (`BUG-034`).
- **Cổng kiểm chứng Wave 3 (`Gate W3`):**
  - Test tự động xác nhận: `bars@08:00:00 >= bars@07:59:59`; `SPY 14:30 UTC` `Once` RSI alert không nổ hồi tố; `1W` giữa tuần không nổ ma; Live alert bị ẩn trong Replay quá khứ; `OutlierImpact` trả `-100.0`; `FPT` Backtest không mở lệnh Short trái phép; `VN30-PIT` tôn trọng `VN30 Only`.
  - `cargo test` đạt `100% PASS`.

---

### WAVE 4 — `IndicatorCache`, Ổn Định Số Học, Bộ Đệm `AVWAP` & Hoàn Thiện Công Cụ Vẽ
- **Mục tiêu:** Đảm bảo chỉ báo luôn khớp tham số, không `NaN`/shimmer, `AVWAP` siêu nhanh `< 1 ms` trên `100K` nến, và `TextNote`/Custom Fib sắc nét không cắt chữ.
- **Danh sách lỗi xử lý (`5` bugs):** `BUG-014` (`P1`), `BUG-015` (`P1`), `BUG-026` (`P2`), `BUG-027` (`P2`), `BUG-033` (`P3`).
- **Hạng mục kỹ thuật cụ thể:**
  1. **Sửa triệt để `IndicatorCache`** ([`src/indicators/mod.rs:1090-1357`](../../src/indicators/mod.rs)):
     - Lưu riêng cấu hình/chữ ký đã tính (`computed_sma_cfg`, `computed_ema_cfg`, `computed_atr_cfg`...) cho từng chỉ báo thay vì dùng chung `last_config`, đảm bảo sửa tham số khi đang tắt rồi bật lại luôn tính ngay kết quả mới (`SMA(20) 169.50 -> SMA(50) 154.50`) và `PAT` luôn có `ATR` mới nhất (`BUG-014`).
  2. **Lưu đệm `AnchoredVwap (AVWAP)` trong `R11RenderMemo`** ([`src/drawings/geometry.rs:761-813`](../../src/drawings/geometry.rs), [`src/chart.rs:1898, 2721, 12057`](../../src/chart.rs)):
     - Cache chuỗi Welford `AVWAP` theo khóa `(drawing_id, anchor_bar, data_revision, candle_len)` dùng chung giữa pha hit-test và pha vẽ -> đưa thời gian CPU hover `100K` nến + `2 AVWAP` xuống **`< 1.0 ms`** (`BUG-015`).
  3. **Ổn định số học, `Conflation` & định dạng biểu đồ** ([`src/indicators/extended.rs`](../../src/indicators/extended.rs), [`src/indicators/mod.rs:737-741`](../../src/indicators/mod.rs), [`src/viewport/candle_metrics.rs:149-153`](../../src/viewport/candle_metrics.rs), [`src/chart.rs:4637-4892, 6034, 6516, 8534-8538, 9858-9860`](../../src/chart.rs)):
     - Chặn `NaN`/`±Inf` ở `MFI`, `Volume SMA`, `SuperTrend`, `Keltner`, `CCI`, và bỏ qua volume `NaN` khi cộng gộp `CandleBlock` (`BUG-027`).
     - Chế độ `%` và `Idx100` tự tìm thanh nến đầu tiên có `close.is_finite() && close > 0.0` làm gốc; căn pha lấy mẫu gộp theo lưới cố định tránh rung khi pan `1` thanh; ngắt dải băng `BB`/`Keltner`/`VWAP` tại điểm `None` (`BUG-027`).
     - Sửa định dạng volume `< 1.000` giữ nguyên số nguyên (không in `0K`), hỗ trợ đủ bậc `K/M/B/T` cho HUD và `CVD`; `0.00%` không có dấu `+`; kẹp nhãn `FRVP POC` theo `plot_rect.top()`; lưu `invert_scale` trong `WorkspaceSlotTemplate` (`BUG-033`).
  4. **Sửa `TextNote` UTF-8 `280` ký tự & giãn cách nhãn Custom Fibonacci** ([`src/drawings/geometry.rs:831-842`](../../src/drawings/geometry.rs), [`src/chart.rs`](../../src/chart.rs)):
     - Dùng đo bề rộng pixel thực tế (`egui::Fonts::layout_job` / galley wrap) cho `TextNote` để hiển thị trọn vẹn `280` ký tự tiếng Việt/CJK/emoji không bị cắt tại `272` ký tự (`BUG-026`).
     - Thêm chống đè dọc (`min_gap >= 14 px`) cho nhãn tỷ lệ Custom Fib (`ICT OTE 9` / `Full 13`) để không chồng chữ lên nhau (`BUG-026`).
- **Cổng kiểm chứng Wave 4 (`Gate W4`):**
  - Test hồi quy xác nhận: `SMA(20)->tắt->SMA(50)->bật == 154.50`; `100K` + `2 AVWAP` CPU hover `< 1.5 ms`; `MFI`/`CCI`/`SuperTrend`/`Keltner` không bao giờ sinh `NaN`/`Inf`; `TextNote` `280` ký tự tiếng Việt giữ nguyên `100%` ký tự; `WorkspaceSlotTemplate` bảo toàn `invert_scale`.
  - `cargo test` đạt `100% PASS`.

---

### WAVE 5 — TradingView UX Parity, Watchlist `471` Mã `60 FPS`, `RRG`, `Anti-Collision`, Modals & `2x2`
- **Mục tiêu:** Đưa trải nghiệm hình ảnh và độ mượt tương tác trên `471` mã thực tế ngang tầm TradingView Desktop ở cả `1280×800` và `1440×1000`.
- **Danh sách lỗi xử lý (`7` bugs):** `BUG-018` (`P2`), `BUG-019` (`P2`), `BUG-020` (`P2`), `BUG-022` (`P2`), `BUG-023` (`P2`), `BUG-024` (bố cục DOM `1280×800`), `BUG-032` (`P3` phần UI).
- **Hạng mục kỹ thuật cụ thể:**
  1. **Chống đè trục giá phải & nhãn trên biểu đồ (`TC-11`)** ([`src/chart.rs:11569-11582`](../../src/chart.rs)):
     - Đưa toàn bộ nhãn giá trục phải của `Alert` (`paint_alert_price_line_and_pill`) và `Drawing` đang chọn (`H-Line`, `Pos R:R`, `FRVP`, `AVWAP`) đi qua chung `resolve_non_overlapping_axis_pill_y` với `Last`, `Entry`, `SL`, `TP`.
     - Dịch ngang hoặc xếp tầng nhãn `Alert` trên mặt biểu đồ để không đè lên thanh `TP`/`SL`/`LONG 200` PnL (`BUG-018`).
  2. **Sửa độ mờ xuyên thấu & căn lề hộp thoại `Templates` / `Cmd+K`** ([`src/app.rs:18816-18838, 19032-19053`](../../src/app.rs)):
     - Thêm `.fade_in(false)` cho `render_templates_modal`, `render_command_palette_modal` (và rà soát toàn bộ modal khác trong `src/app.rs`).
     - Đặt giới hạn bề rộng + tự ngắt dòng (`wrap`) cho mô tả preset trong `Templates` để khung giữ đúng `modal_w` và nút `Close (Esc)` căn sát mép phải tiêu đề (`BUG-019`).
  3. **Ảo hóa Watchlist `471` mã `60 FPS`, khử trùng dữ liệu giả & chống đè `RRG`** ([`src/app.rs:4710-4841, 13799-13804`](../../src/app.rs)):
     - Thay vòng lặp vẽ toàn bộ `484` thẻ Watchlist bằng `ScrollArea::vertical().show_rows(...)` -> giảm CPU Watchlist từ `3.37 ms` xuống **`< 0.6 ms`** (`BUG-020`).
     - Khử trùng: khi đang nạp `CustomFile` (`CafeF 471 mã`), tách biệt hoàn toàn với `13` mã mẫu giả lập (không còn `2` dòng `FPT`, không còn `VN30(16)`, không hiện ngày `2026-11-13` hay `FPT +2.52%` giả lập cạnh `FPT -1.17%` thật).
     - Tối ưu Bar Replay `+1` khi Watchlist mở (tính lười/theo nhóm hiển thị hoặc cập nhật tăng dần `O(visible_rows)` thay vì quét lại cả `484` mã đồng bộ trên luồng UI) -> giảm từ **`139.4 ms` xuống `< 16.0 ms` (`>= 60 FPS`)**, và xây chỉ mục `Cmd+K` mượt `< 15 ms` (`BUG-020`).
     - Chống đè nhãn `RRG`: chỉ hiển thị nhãn cho top `12–16` mã nổi bật / rổ `VN30` / mã đang chọn + mã đang hover chuột kèm thuật toán né đè chữ (thay vì vẽ chồng cả `483` tên mã thành mảng trắng đặc, `BUG-020`).
  4. **Sửa Lưới Đa Biểu Đồ `2x2` & Crosshair vùng trắng tương lai** ([`src/chart.rs:3493-3495, 6687-6705`](../../src/chart.rs), [`src/viewport/multi_chart.rs:213-260`](../../src/viewport/multi_chart.rs), [`src/app.rs`](../../src/app.rs)):
     - Duy trì hiển thị linked crosshair (`Sync X`) trên các ô phụ khi chuột di chuyển sang vùng trắng tương lai (chiếu theo khoảng thời gian tương ứng thay vì trả `None`); sửa méo `Sync T` trên `85%` whitespace (`BUG-022`).
     - Khi chuyển sang `2x2` từ `CustomFile`, mặc định nạp `S2: VNINDEX` từ `data/raw/cafef/CafeF.INDEX.Upto06112026.csv` (sạch, không gai `-34.5%`) và `S3: VCB` từ chính bộ `CustomFile` đang mở (không nạp `VN30 Basket` cũ trống `95%` màn hình, `BUG-022`).
  5. **Sửa Top Toolbar (`%Chg`, tiền tệ, gọn `<= 2` hàng ở `1280×800`), DOM centering & Polish `P3`** ([`src/app.rs`](../../src/app.rs), [`src/model.rs:29-41`](../../src/model.rs)):
     - Sửa `%Chg` trên thanh tiêu đề sang `(close - prev_close) / prev_close` khớp Watchlist và chuẩn TradingView (`BUG-023`).
     - Hiển thị đơn vị tiền tệ theo sàn (`kVND` / `₫` cho `HOSE`/`HNX`/`UPCOM`/`VN30F1M`; `$` cho US/Crypto) trên thanh trạng thái, Paper Trading và DOM (`BUG-023`).
     - Gọn hóa thanh công cụ trên cùng tối đa `2` hàng ở `1280×800` (không còn `Cmd+K`, `✓ Volume`, `-> Order Ticket` rớt hàng mồ côi); tự xóa `symbol_picker_filter` ngay sau khi chọn mã (`BUG-023`).
     - Tự cuộn căn giữa dòng `Mark` trong DOM Ladder tại `1280×800` để thấy đồng thời cả Ask, Mark và Bid (`BUG-024`).
     - Hoàn thiện `P3` (`BUG-032`): sửa `"144 dates"` -> `"151 dates"` (hoặc số lượng động từ `VIETNAM_HOLIDAYS_V1.len()`); hiển thị chú thích phân trang `"64 / 8026"` ở tab `Sig`; hiện banner phản hồi xung đột khi gán phím trong `[Keys]`; vô hiệu hóa nút `-> Order Ticket` khi đang chọn `TextNote`; bổ sung nút chọn ngôn ngữ `EN / VI / Both` trong `Signal Catalog`; đệm biên trên cho nhãn `PAT` và tránh cắt cụt chú giải `ICH` / `Multi-Strategy`.
- **Cổng kiểm chứng Wave 5 (`Gate W5`):**
  - Đo hiệu năng thực tế: Watchlist `471` mã `< 1.5 ms`/frame; Replay `+1` kèm Watchlist mở `< 16.0 ms`; `Cmd+K` mở `< 20 ms`.
  - Chụp ảnh Metal GPU kiểm chứng từng màn hình (`00`, `07`, `08`, `09`, `10`, `12`, `14`, `15`, `16`, `17`, `22`) và soi trực tiếp bằng `read_file`.

---

### WAVE 6 — Đóng Gói `.app` Độc Lập, Benchmark, Tái Kiểm Toán Toàn Bộ `TC-01..TC-65` & Xuất Báo Cáo Ký Duyệt `GO`
- **Mục tiêu:** Đóng gói hoàn chỉnh bản phát hành macOS, tái chạy toàn bộ 3 lớp kiểm thử và xác nhận `65/65 PASS`.
- **Danh sách lỗi xử lý:** `BUG-032` (phần đóng gói `Info.plist` / `Cargo.toml` / `.app` độc lập) + tái xác minh `BUG-001..BUG-034`.
- **Hạng mục kỹ thuật cụ thể:**
  1. Đồng bộ phiên bản giữa `Cargo.toml` và [`scripts/package_macos_app.sh`](../../scripts/package_macos_app.sh) (`Info.plist`), hỗ trợ `.app` tự phát hiện thư mục dữ liệu người dùng/repo linh hoạt hơn khi chạy ngoài thư mục mã nguồn (`BUG-032`).
  2. Biên dịch bản phát hành (`cargo build --release`) và chạy `./scripts/package_macos_app.sh` tạo lại `/Users/mizuhara/workspace/sumi/dist/Sumi Workstation.app` (kiểm tra `codesign -dv --verbose=4` và chạy thử từ `cwd = /tmp`).
  3. Chạy kiểm thử toàn diện:
     - `cargo test` (`100% PASS`, `0` fail).
     - `viewport_bench` với cả 4 cờ `--r12`, `--r17`, `--r18`, `--r19` trong thư mục cách ly (giữ sạch `docs/evidence/` lịch sử): xác nhận `overall_pass: true` (`p99 < 4.0 ms`).
     - Chạy lại toàn bộ probe đối kháng (`TC-01..TC-65`) và chụp lại ảnh Metal GPU (`2560×1600` & `2880×2000`) vào `docs/testing/qa_evidence_after/` -> dùng `read_file` soi pixel xác nhận từng ảnh đã sạch lỗi.
  4. Kiểm toán bảo toàn dữ liệu gốc (`TC-65`): chạy `shasum -a 256 /Users/mizuhara/workspace/work/CafeFData/*.zip` và `git status` xác nhận `0` byte sửa đổi trong `CafeFData/*.zip`, `ARCHIVE/`, `docs/evidence/`.
  5. Xuất báo cáo tái kiểm thử cuối cùng tại `docs/testing/QA_REAUDIT_REPORT_R19_PATCH.md` (bảng so sánh Trước/Sau từng mã `BUG-001..BUG-034`, ma trận `65/65 PASS`, số liệu benchmark và phán quyết **`GO FOR v1.0`**).

---

## 2. Bảng Truy Xuất Toàn Bộ `34` Lỗi -> Wave Thực Thi (Bảo Đảm Không Sót Lỗi Nào)

| Wave | Danh Sách Bug Đảm Nhận | Tổng Số |
| :--- | :--- | :---: |
| **Wave 1** | `BUG-001`, `BUG-002`, `BUG-004`, `BUG-008`, `BUG-009`, `BUG-017`, `BUG-024` (math/tier), `BUG-030`, `BUG-034` (holidays/DOM/store/SL Adh) | `9` |
| **Wave 2** | `BUG-003`, `BUG-005`, `BUG-006`, `BUG-007`, `BUG-012`, `BUG-013`, `BUG-021`, `BUG-025`, `BUG-031` | `9` |
| **Wave 3** | `BUG-010`, `BUG-011`, `BUG-016`, `BUG-028`, `BUG-029`, `BUG-034` (backtest/alerts/catalog/md) | `5 (+1 phần)` |
| **Wave 4** | `BUG-014`, `BUG-015`, `BUG-026`, `BUG-027`, `BUG-033` | `5` |
| **Wave 5** | `BUG-018`, `BUG-019`, `BUG-020`, `BUG-022`, `BUG-023`, `BUG-024` (UI 1280×800), `BUG-032` (UI copy) | `6 (+1 phần)` |
| **Wave 6** | `BUG-032` (packaging `.app`) + Re-Audit toàn diện `BUG-001..BUG-034` & `TC-01..TC-65` | `Toàn bộ 34` |
