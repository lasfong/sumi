# BÁO CÁO KIỂM TOÁN TOÀN DIỆN DỮ LIỆU, TÍNH NĂNG & KHOẢNG CÁCH VỚI TRADINGVIEW CỦA SUMI SUPERCHARTS (`v2.2.0`)

- **Ngày kiểm toán:** `2026-11-27`
- **Phiên bản mã nguồn kiểm toán:** Sumi Supercharts `v2.2.0` (`Cargo.toml:10`, `ui/package.json:4`)
- **Phạm vi kiểm chứng trực tiếp (100% Read-Only):**
  - Toàn bộ mã nguồn Rust workspace: `crates/sumi-data/`, `crates/sumi-core/` (`83.870` dòng Rust), `crates/sumi-desktop/` (`45` Tauri commands)
  - Toàn bộ mã nguồn Frontend React/TypeScript: `ui/src/` (`29` tệp nguồn, nền tảng biểu đồ `klinecharts@^10.1.1`)
  - Mã nguồn thư viện phụ thuộc thượng nguồn: `tradingview-rs v0.5.0` (`rev 5794fe4` tại `~/.cargo/git/checkouts/tradingview-rs-791a80f0be896def/5794fe4/`)
  - Hợp đồng liên kết hệ thống ngoài: `/Users/mizuhara/workspace/yumeko/crates/yumeko-sumi-contract/`
  - Cơ sở dữ liệu thực tế đang vận hành (mở ở chế độ `mode=ro` tuyệt đối):
    - `~/Library/Application Support/com.sumi.supercharts/sumi_market.db` (`84.40 MiB` — `958.282` nến)
    - `~/Library/Application Support/com.sumi.supercharts/sumi_workspace.db` (`76 KiB` + `237 KiB` WAL)
    - Đối chiếu chéo kho thô `/Users/mizuhara/workspace/sumi/data/raw/cafef/` & `/Users/mizuhara/workspace/sumi/data/cache/sumi_market.db`

---

## TÓM TẮT ĐIỀU HÀNH (`EXECUTIVE SUMMARY`)

1. **Thực trạng Dữ liệu (`Data Reality`):**
   - Sumi vận hành theo kiến trúc **Hybrid Local-First SQLite (`sumi_market.db`)** kết hợp **2 nguồn cấp thực tế**: (a) **CafeF CSV lịch sử dài hạn** (`2000–2026` cho chỉ số, `2016–2026` cho `471` cổ phiếu chọn lọc) và (b) **TradingView WebSocket/REST (`tradingview-rs`) ở chế độ Khách ẩn danh (`unauthorized_user_token`)**.
   - Trên toàn bộ sàn chứng khoán Việt Nam (`HOSE`, `HNX`, `UPCOM`, `VNINDEX`, `VN30F1!`), luồng TradingView ẩn danh bị **trễ 15 phút (`delay_secs = 900`)**, trong khi Crypto (`BINANCE`) và cổ phiếu Mỹ (`NASDAQ`) có độ trễ `0s`.
   - Trong `sumi_market.db` thực tế hiện tại (**`958.282` nến / `486` mã**): **100% nến đều lưu ở chế độ `adjustment = 'splits'`** (không có bất kỳ dòng giá gốc `unadjusted` nào dù file `CafeF.SolieuGD.Raw.Upto06112026.csv` nặng `940.271` dòng đang nằm sẵn trên ổ đĩa nhưng bị khóa `include_raw: false`). Đặc biệt, **`469 / 471` cổ phiếu Việt Nam (`99,58%`) mới chỉ có khung `1D`**, hoàn toàn chưa có nến intraday trong DB; đồng thời **`435 / 475` chuỗi VN `1D` (`91,6%`) đang dừng ở ngày `2026-11-06`**, gây ra **"Bẫy dữ liệu EOD cũ" (`Stale EOD Trap`)** khi quét `Stock Screener`.
   - Thư viện `tradingview-rs v0.5.0` đang cài sẵn trong máy thực chất chứa một **"mỏ vàng" chưa khai thác**: hơn **`1.340` chỉ tiêu Báo cáo Tài chính (`STD;Fund_*` theo Quý/Năm/TTM)** chạy được ngay không cần tài khoản trả phí, bộ lọc toàn cầu `list_symbols` lấy đủ `~1.600+` mã VN, thực thi Pine/Study trên máy chủ TradingView, Tin tức doanh nghiệp và Lịch kinh tế vĩ mô.
   - Giữa **Sumi** và **Yumeko (`yumeko-sumi-contract`)** hiện **hoàn toàn chưa có cầu nối dữ liệu hay IPC nào** — Sumi chưa tiếp cận được dữ liệu khớp lệnh từng tick (`Time & Sales`), Mua/Bán chủ động thật, Sổ lệnh Level 2 hay Khối ngoại/Tự doanh của thị trường Việt Nam.

2. **Thực trạng Tính năng (`Codebase vs. UI Reality`):**
   - Giao diện React/KLineChart `v2.2.0` đã hoàn thiện rất tốt chuỗi công việc cốt lõi: **Đa biểu đồ `1..4` ô**, **37 công cụ vẽ PTKT** (có lọc khung giờ, Magnet, Undo/Redo), **16 chỉ báo chọn lọc chuẩn QMV** (đặc biệt là **Nón xác suất tương đồng lịch sử `AFC`** tính bằng Rust), **Causal Multi-Timeframe Bar Replay**, **Paper Broker chuẩn VSD `T+2` FIFO & Phái sinh `VN30F1!`**, **Stock Screener `123` tín hiệu**, và **Strategy Tester** (12 chiến lược + Rule Builder + Danh mục đa mã + Walk-Forward + Monte Carlo).
   - Tuy nhiên, có sự **lệch pha đáng kể giữa lõi Rust `sumi-core` (`83.870` LOC kế thừa từ thế hệ workstation trước) và tầng giao diện React `ui/`**: hàng loạt module đẳng cấp cao đã viết xong và có unit test trong `sumi-core` nhưng **bị bỏ ngỏ chưa nối lên Tauri/UI**, bao gồm: **TPO Market Profile**, **Anchored VWAP**, **CVD**, **5 chỉ báo (`SuperTrend`, `PSAR`, `Keltner`, `Stochastic`, `MFI`)**, **Nhận diện mẫu nến Nhật**, **Độ rộng thị trường (`Market Breadth`) + Biểu đồ xoay dòng tiền `RRG` 4 góc phần tư**, **AlertEngine cảnh báo trên 123 tín hiệu QMV**, **Thang giá Synthetic DOM / Footprint**, **Bộ lập kế hoạch đi vốn (`Trade Planner`) & Nhật ký tâm lý/sai lầm**, **Bảng chấm điểm cuối phiên Replay (`Session Debrief`)**, và **Trình xuất báo cáo Backtest HTML/PDF chuẩn tổ chức**.

3. **Vị thế cạnh tranh trước TradingView:**
   - Sumi thua TradingView ở các mảng nền tảng đám mây & công cụ biểu đồ nâng cao: ngôn ngữ kịch bản tùy biến (`Pine Script`), biểu đồ phi thời gian (`Renko/Kagi/P&F/TPO/Footprint`), so sánh liên thị trường & biểu đồ công thức (`HOSE:SSI / HOSE:VNINDEX`), `Anchored VWAP / FRVP` kéo thả trực tiếp, sự kiện `D/S/E` & BCTC dưới chân nến, lọc cổ phiếu đa khung thời gian đồng thời, giải ngân/chốt lời từng phần (`Pyramiding / Scale-out`) và `Bar Magnifier` trong Backtest.
   - Ngược lại, **Sumi ĐÃ VƯỢT XA TradingView bản chuẩn ở đặc thù vi cấu trúc TTCK Việt Nam và kiểm định định lượng chống Overfitting**: khóa lô cổ phiếu chờ về **VSD `T+2` (`13:00 ICT`) theo FIFO**, biên độ **Trần/Tham chiếu/Sàn (`±7%/±10%/±15%`)** kèm mô phỏng kẹt thanh khoản **`trắng bên mua`** trong Backtest, bước giá động HOSE (`10/50/100 VND`) + thuế bán `0.10%` + ký quỹ VSD `17%` `VN30F1!`, **Nón xác suất `AFC`**, **Walk-Forward Optimization + Monte Carlo `2.000` kịch bản + Backtest Danh mục đa mã**, và **Bar Replay đa khung nhân quả kèm sổ lệnh Sandbox tách biệt**.

---

# CHƯƠNG 1 — BÁO CÁO KIỂM TOÁN DỮ LIỆU & NGUỒN CẤP CỦA SUMI (`DATA & UPSTREAM AUDIT`)

## 1.1. Nguồn 1 — TradingView Feed (`tradingview-rs v0.5.0`)

### A. Cơ chế kết nối, Xác thực, Giới hạn tải & Độ trễ thực tế

| Hạng mục kiểm toán | Hiện trạng thực tế trong mã nguồn | Tham chiếu `file:line` |
|---|---|---|
| **Phiên bản phụ thuộc** | Crate git `tradingview-rs v0.5.0` (`rev = "5794fe4"`), cấu hình `default-features = false, features = ["rustls-tls"]`. | `crates/sumi-data/Cargo.toml:28-33` |
| **Giao thức kết nối** | - **Lịch sử nến & Streaming nến/giá & Resolve mã:** WebSocket `wss://prodata.tradingview.com/socket.io/websocket` (ưu tiên `DataServer::ProData`, tự động fallback sang `DataServer::Data` nếu lỗi).<br>- **Tìm kiếm mã (`search_symbols`):** HTTP REST `symbol-search.tradingview.com` (dùng client `reqwest` dùng chung). | `crates/sumi-data/src/tradingview_feed.rs:58-63, 184-198, 208-278` |
| **Chế độ xác thực (`Auth`)** | **100% Guest Mode ẩn danh:** Khóa cứng `auth_token("unauthorized_user_token")`. Do tắt `default-features` trong `Cargo.toml`, feature `user` (đăng nhập Cookie + 2FA TOTP của `tradingview-rs`) hiện bị biên dịch loại bỏ hoàn toàn. Không đọc biến môi trường hay cấu hình token người dùng. | `crates/sumi-data/src/tradingview_feed.rs:60`<br>`crates/sumi-data/Cargo.toml:31` |
| **Giới hạn nến mỗi lần tải** | - Yêu cầu mở biểu đồ ban đầu: `1.500` nến (`KLineViewport.tsx:656`).<br>- Kéo lùi quá khứ (`scrollback`): tải từng trang `1.000` nến, tối đa trần cứng **`16.500` nến/chuỗi** (`MAX_SCROLLBACK_DEPTH`). Máy chủ `ProData` của TradingView cấp thực tế khoảng `10.000 – 16.500` nến cho phiên Guest. | `ui/src/chart/KLineViewport.tsx:656`<br>`crates/sumi-data/src/cache_service.rs:654`<br>`tradingview-rs/docs/history-depth.md:58-70` |
| **Giới hạn tần suất (`Rate Limit`)** | Bộ điều tiết kép trong `FeedRateLimiter`:<br>1. **Token Bucket:** `burst_capacity = 6.0` yêu cầu, tốc độ hồi `refill_per_sec = 3.0 req/s`.<br>2. **Semaphore đồng thời:** tối đa `max_concurrent = 4` tác vụ mạng song song.<br>Áp dụng cho `search_symbols`, `resolve_symbol`, `fetch_history` (streaming WebSocket giữ kết nối riêng). | `crates/sumi-data/src/rate_limit.rs:28-38, 62-104` |
| **Độ trễ dữ liệu thực tế (`Delay`)** | Kiểm chứng trực tiếp trường `delay_secs` lưu trong `symbols.metadata_json` của `sumi_market.db`:<br>- **Sàn Việt Nam (`HOSE`, `HNX`, `UPCOM`, `HOSE:VNINDEX`, `HOSE:VN30`, `HNX:HNXINDEX`, `HNX:VN30F1!`):** **`delay_secs = 900` (Trễ 15 phút)** trên feed TradingView ẩn danh.<br>- **Crypto (`BINANCE:BTCUSDT`, `ETHUSDT`...):** **`delay_secs = 0` (Realtime)**.<br>- **Cổ phiếu/ETF Mỹ (`NASDAQ:AAPL/MSFT/NVDA`, `AMEX:SPY`):** **`delay_secs = 0`** (CBOE BZX realtime trên TV).<br>- **Forex & Vàng (`FX_IDC:USDVND`, `OANDA:XAUUSD`):** **`delay_secs = 0` (Realtime)**. | `sumi_market.db -> symbols.metadata_json`<br>`crates/sumi-data/src/tradingview_feed.rs:321` |

### B. Các kiểu dữ liệu & Khung thời gian thực tế lấy được từ TradingView

1. **Nến OHLCV lịch sử (`fetch_history`):**
   - Hỗ trợ **9 khung thời gian** trong tầng `sumi-data` và trên giao diện `TopHeader`: `1m`, `5m`, `15m`, `30m`, `1h`, `4h`, `1D`, `1W`, `1M` (`crates/sumi-data/src/types.rs:108-120`, `ui/src/types/market.ts:3`).
   - Trường dữ liệu mỗi nến (`OhlcvBar` tại `types.rs:697-715`): `timestamp` (UTC ms), `open`, `high`, `low`, `close`, `volume`, và `turnover` (tính xấp xỉ nội bộ `close * volume` khi lưu, không phải giá trị giao dịch khớp lệnh chính xác từng tick của Sở).
   - **Phát hiện lệch khung giữa `sumi-data` và `sumi-core`:** Hàm `Interval::to_core_timeframe()` (`crates/sumi-data/src/types.rs:168-180`) ánh xạ **`Interval::M30 => None`** và **`Interval::MN1 => None`** vì `sumi_core::Timeframe` (`crates/sumi-core/src/data/timeframe.rs:20-29`) chỉ định nghĩa 7 khung (`M1, M5, M15, H1, H4, D1, W1`). Hệ quả: xem biểu đồ `30m` và `1M` vẫn chạy bình thường, nhưng các tính năng lõi phụ thuộc `Timeframe` của `sumi-core` (như kiểm định chiến lược `Strategy Tester` trên `30m`/`1M`) không hỗ trợ hai khung này (`commands/backtest.rs:195-207`).

2. **Luồng nến trực tiếp (`subscribe_bars`):**
   - Mở phiên WebSocket `ChartSession` trực tiếp cho từng ô biểu đồ (`crates/sumi-data/src/tradingview_feed.rs:582-732`), tự động nối lại với backoff mũ (`1s -> 30s`) khi đứt mạng, đẩy từng nhịp cập nhật nến đang chạy về UI qua `tauri::ipc::Channel<LiveBarPayload>` và đồng thời ghi đè vào RAM LRU (`RamBarCache`, tối đa `96` chuỗi × `20.000` nến tại `ram_cache.rs:15-20`) + SQLite (`source = 'tradingview_live'`).

3. **Luồng bảng giá (`subscribe_quotes`):**
   - Đăng ký `QuoteSession` lấy 10 trường từ TradingView (`tradingview_feed.rs:762-773`): `lp`, `ch`, `chp`, `volume`, `open_price`, `high_price`, `low_price`, `prev_close_price`, `bid`, `ask`.
   - **Điểm nghẽn gọt dữ liệu:** Khi đẩy lên UI qua `LiveQuotePayload` (`crates/sumi-desktop/src/commands/market.rs:134-145`), backend **lược bỏ mất 5 trường `bid`, `ask`, `open_price`, `high_price`, `low_price`**, chỉ còn giữ `last_price`, `prev_close`, `change`, `change_percent`, `volume`.
   - **Phát hiện lỗi nghiêm trọng (`Wire Alias Bug` cho Phái sinh `HNX:VN30F1!`):**
     - Khi gọi đi, `map_to_tv_wire_symbol` (`tradingview_feed.rs:1052-1071`) dịch `HNX:VN30F1!` sang mã dây TradingView là `HNX:VN301!`.
     - Ở chiều `resolve_symbol`, `cache_service.rs:248-253` có dịch ngược lại thành `HNX:VN30F1!`.
     - Nhưng ở luồng streaming bảng giá `QuoteStreamHandler::handle_quote_data` (`tradingview_feed.rs:819-839`), mã trả về từ TradingView vẫn giữ nguyên là **`HNX:VN301!`** mà không dịch ngược về `HNX:VN30F1!`! Do Watchlist trên UI lưu khóa `quotesMap["HNX:VN30F1!"]`, các gói tin giá nhảy trực tiếp của `VN30F1!` bị lệch khóa trên bảng Watchlist!

4. **Tìm kiếm (`search_symbols`) & Phân giải Metadata (`resolve_symbol`):**
   - Hỗ trợ tìm kiếm đa lớp tài sản (`all`, `vn`, `stocks`, `indices`, `futures`, `crypto`, `forex`, `macro` tại `commands/market.rs:321-418`), có bộ đệm cục bộ thông minh khi mất mạng.
   - `ResolvedSymbol` (`crates/sumi-data/src/types.rs:758-785`) trích xuất: `ticker`, `exchange`, `listed_exchange`, `symbol`, `description`, `market_type`, `currency`, `timezone`, `session`, `price_precision`, `pricescale`, `minmov`, `variable_tick_size` (kèm bảng bậc thang bước giá `VariableTickTable`), `point_value`, `delay_secs`, `isin`.

### C. Các lớp tài sản / Thị trường lấy được
- **Cổ phiếu Việt Nam:** `HOSE:*`, `HNX:*`, `UPCOM:*` (trễ 15p từ TV; lịch sử EOD sâu từ CafeF).
- **Chỉ số Việt Nam:** `HOSE:VNINDEX`, `HOSE:VN30`, `HNX:HNXINDEX`, `UPCOM:UPINDEX` (`map_to_tv_wire_symbol` hỗ trợ ánh xạ các biến thể tên mã tại `tradingview_feed.rs:1052-1071`).
- **Phái sinh Việt Nam:** `HNX:VN30F1!` (hợp đồng tương lai chỉ số VN30 kỳ hạn gần nhất, nối sang mã `HNX:VN301!` của TradingView).
- **Tài sản Quốc tế & Vĩ mô:** Crypto (`BINANCE:BTCUSDT`, `OKX:*`, `BYBIT:*`), Cổ phiếu & ETF Mỹ (`NASDAQ:*`, `NYSE:*`, `AMEX:SPY`), Ngoại hối (`FX_IDC:USDVND`, `FX:EURUSD`), Hàng hóa (`OANDA:XAUUSD`, `TVC:USOIL`), Chỉ số Vĩ mô (`TVC:DXY`, `TVC:US10Y` — tải tốt qua `search_symbols` / `get_bars` khi online).

### D. Khám phá "Mỏ vàng tiềm năng ẩn" trong `tradingview-rs v0.5.0` mà Sumi CHƯA GỌI TỚI

Kiểm tra trực tiếp mã nguồn crate `tradingview-rs` đang cài tại `~/.cargo/git/checkouts/tradingview-rs-791a80f0be896def/5794fe4/crates/tradingview/src/` cho thấy Sumi mới chỉ khai thác khoảng **~25% bề mặt API** của thư viện này. Dưới đây là **8 nhóm năng lực lớn đang nằm sẵn trong dependency** mà Sumi chưa sử dụng:

| # | Module có sẵn trong `tradingview-rs` | Dữ liệu / Tính năng lấy được | Chạy với Guest Token? | Ý nghĩa nếu mở khóa cho Sumi |
|---|---|---|---|---|
| 1 | **`fundamental` module** (`src/fundamental/{mod,full,registry}.rs`) | Hơn **`1.340` chỉ tiêu Báo cáo Tài chính** (`STD;Fund_*` & `STD;CryptoFund_*`) theo **Năm (`FY`), Quý (`FQ`), Bán niên (`FH`), `TTM`**: Doanh thu, LNST, Biên lợi nhuận, EPS, P/E, P/B, ROE, ROA, Nợ/VCSH, Lưu chuyển tiền tệ, Cổ tức tiền mặt (`fetch_full_fundamentals`). | **CÓ (`100%`)** | Giải quyết ngay bài toán thiếu dữ liệu cơ bản (`P/E, P/B, EPS, ROE, Tăng trưởng LNST`) cho cổ phiếu VN mà **không tốn phí mua nguồn BCTC bên thứ 3**! |
| 2 | **Các trường giàu thông tin trong `SymbolInfo`** (`src/chart/models.rs:359-443`) | Mỗi lần Sumi gọi `resolve_symbol`, TradingView thực chất đã trả sẵn `sector`, `industry`, `market_cap_basic`, `price_earnings_ttm`, `last_annual_eps`, `total_shares_outstanding_calculated`, `total_revenue`, `earnings_release_date`, `business_description`, `founded` — nhưng `from_tv_symbol_info` (`sumi-data/src/types.rs:788-843`) **đang vứt bỏ sạch các trường này**! | **CÓ (`100%`)** | Chỉ cần giữ lại các trường này là có ngay **Vốn hóa, P/E TTM, EPS, Ngành, Ngày công bố KQKD dự kiến & Mô tả doanh nghiệp** cho thẻ Chi tiết mã và Stock Heatmap! |
| 3 | **`list_symbols` & `advanced_search_symbol`** (`src/client/misc.rs:140-260`) | Liệt kê phân trang toàn bộ mã chứng khoán theo Quốc gia (`Country::VN`), Sàn, Ngành với `30` luồng bất đồng bộ. | **CÓ (`100%`)** | Tự động đồng bộ danh mục **toàn bộ `~1.600+` mã `HOSE + HNX + UPCOM`**, xóa bỏ giới hạn `471` mã tĩnh của file CafeF hiện tại! |
| 4 | **`study` module (`StudyClient`)** (`src/study/client.rs`, `src/chart/study.rs`) | Liệt kê chỉ báo tích hợp (`get_builtin_indicators`), tìm kiếm chỉ báo cộng đồng (`search_indicator`), lấy metadata đầu vào và chạy trực tiếp `StudyConfiguration::{Builtin, Pine}` trên máy chủ TradingView qua WebSocket (bao gồm cả nhận diện mẫu nến & mẫu hình biểu đồ của TV). | **CÓ** (với Builtin & Public Pine) | Cho phép đối chiếu chéo hoặc chạy các study phức tạp của TradingView mà không phải tự viết lại từ đầu. |
| 5 | **`get_technical_analysis` & Scanner API** (`src/client/misc.rs:32-98`) | Gọi `scanner.tradingview.com` lấy điểm xếp hạng PTKT đa khung giờ (`1m` -> `1M`: Oscillators / Moving Averages Summary) và hạ tầng quét đám mây. | **CÓ (`100%`)** | Có thể bổ sung giải pháp **Quét nhanh EOD/Intraday toàn sàn VN từ đám mây TradingView** trong `< 1 giây` mà không lo SQLite cục bộ bị cũ! |
| 6 | **`client::news`** (`src/client/news.rs`) | Lấy danh sách tiêu đề & toàn văn tin tức tài chính theo mã hoặc khu vực (`FinancialStatement`, `PressRelease`, `InsiderTrading`, `CorpActivitiesAll`). | **CÓ (`100%`)** | Bổ sung tab Tin tức & Sự kiện doanh nghiệp ngay trong thanh bên phải (`RightWidgetBar`). |
| 7 | **`client::fin_calendar`** (`src/client/fin_calendar.rs`) | Lịch kinh tế vĩ mô toàn cầu (`actual`, `forecast`, `previous`, mức độ quan trọng `importance`, lọc theo quốc gia/mốc ngày). | **CÓ (`100%`)** | Bổ sung Lịch sự kiện vĩ mô (`CPI, Non-farm, FOMC, Lãi suất SBV`) cho nhà đầu tư vĩ mô/phái sinh. |
| 8 | **Khung thời gian phụ, Biểu đồ phi thời gian & Feature `user`** (`src/models/mod.rs:548-590`, `src/chart/models.rs:10-30`) | Hỗ trợ sẵn `1s, 5s, 10s, 15s, 30s, 3m, 45m, 2h, 3M, 6M, 12M`; kiểu biểu đồ server-side `Renko, LineBreak, Kagi, PointAndFigure, Range`; `Adjustment::Dividends`; và feature `user` đăng nhập tài khoản TradingView cá nhân (`UserCookies` + TOTP) để mở khóa lịch sử intraday sâu `20.000–40.000` nến. | Một phần (giây & lịch sử sâu cần feature `user` + tài khoản TV trả phí) | Mở đường cho khung `2h / 45m / 3M`, biểu đồ `Renko / Kagi`, và cho phép người dùng có sẵn tài khoản TradingView Plus/Premium đăng nhập để lấy thêm lịch sử nến. |

---

## 1.2. Nguồn 2 — Dữ liệu Lịch sử CafeF (`cafef_importer.rs`) & Cơ chế Ghép nối (`Splice`)

### A. Các định dạng CSV hỗ trợ (`crates/sumi-data/src/cafef_importer.rs:230-296`)
Bộ nạp CafeF đọc các tệp CSV định dạng AmiBroker (`<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>`), tự động phân loại theo tên tệp:
1. **`CafeF.INDEX*.csv`:** Chuỗi chỉ số thị trường (`VNINDEX -> HOSE:VNINDEX`, `HNX-INDEX -> HNX:HNXINDEX`, `UPCOM-INDEX -> UPCOM:UPINDEX`), hệ số nhân giá **`1.0x`**, nhãn `source = 'cafef_index'`, `adjustment = 'splits'`.
2. **`CafeF.SolieuGD.Adjusted*.csv`:** Giá cổ phiếu đã điều chỉnh chia tách/cổ tức của CafeF, nhân hệ số **`1.000x`** (đổi từ nghìn đồng `kVND` sang `VND` nguyên), nhãn `source = 'cafef_adjusted'`, `adjustment = 'splits'`.
3. **`CafeF.SolieuGD.Raw*.csv`:** Giá cổ phiếu gốc chưa điều chỉnh, nhân hệ số **`1.000x`**, nhãn `source = 'cafef_raw'`, `adjustment = 'unadjusted'`.

> **Phát hiện quan trọng về Dữ liệu Giá Gốc (`Raw / Unadjusted`):**
> Trong thư mục dự án `/Users/mizuhara/workspace/sumi/data/raw/cafef/` hiện **CÓ SẴN** tệp `CafeF.SolieuGD.Raw.Upto06112026.csv` chứa tới **`940.271` dòng giá gốc chưa điều chỉnh**! Tuy nhiên:
> - `CafeFImportConfig::default()` (`cafef_importer.rs:34`) đặt `include_raw: false`.
> - Lệnh Tauri `import_cafef_dir` trên giao diện (`crates/sumi-desktop/src/commands/market.rs:1116`) cũng khóa cứng `include_raw: false` (chỉ có công cụ dòng lệnh `sumi-data-cli import-cafef --include-raw` tại `src/bin/sumi-data-cli.rs:112` mới bật được cờ này).
> - Do đó, **toàn bộ `940.271` dòng giá gốc bị bỏ qua hoàn toàn** khi người dùng bấm nạp dữ liệu trên ứng dụng!

### B. Cơ chế Ghép nối Tỷ lệ (`Ratio-Splicing`) giữa CafeF & TradingView (`crates/sumi-data/src/storage.rs:228-296, 351-400, 500-528`)
Khi Sumi tải dữ liệu `1D` mới từ TradingView đè lên một mã đã có sẵn lịch sử CafeF trong SQLite:
1. Hệ thống tìm phiên giao dịch trùng ngày gần nhất giữa chuỗi mới từ TradingView (`tv_close`) và dòng `cafef_adjusted` sẵn có trong SQLite (`cf_close`).
2. Nếu độ lệch giá `|tv_close - cf_close| / tv_close > 0.008` (`0.8%` — xảy ra khi mã vừa có đợt chia cổ tức/thưởng cổ phiếu mới sau ngày chốt file CSV CafeF), hệ thống tính `ratio = tv_close / cf_close` (chặn an toàn trong khoảng `[0.02, 50.0]`).
3. Toàn bộ các nến lịch sử cũ trước vùng phủ của TradingView có `source = 'cafef_adjusted'` được nhân `open, high, low, close * ratio`, chia `volume / ratio`, và đổi nhãn thành `source = 'cafef_spliced'`. Sau đó các nến mới từ vùng giao thoa trở đi được ghi đè bằng nến chuẩn của TradingView (`source = 'tradingview'`).

**2 Lỗi Biên (`Edge-Case Bugs`) phát hiện trong thuật toán `Ratio-Splicing`:**
1. **Quên điều chỉnh `turnover` (`storage.rs:274-286`):** Câu lệnh `UPDATE bars SET open = open * ?1, ..., volume = volume / ?1` không cập nhật cột `turnover` (vốn có thể chênh lệch hoặc làm sai lệch bộ lọc thanh khoản lịch sử nếu dùng `turnover`).
2. **Lỗi không thể Splice lần thứ 2 (`storage.rs:265, 290`):** Câu lệnh cập nhật chỉ lọc `WHERE ... AND source = 'cafef_adjusted'` rồi đổi nhãn dòng đã nhân thành `'cafef_spliced'`. Nếu 6 tháng sau cổ phiếu đó **chia tách thêm một lần nữa**, các dòng `'cafef_spliced'` cũ (`2016–2024`) sẽ **không được nhân tỷ lệ mới**, tạo ra một "vết gãy giá" nhân tạo tại ranh giới năm `2025`! (Sửa rất dễ: lọc `source IN ('cafef_adjusted', 'cafef_spliced')`).

### C. Xử lý `Adjustment::Splits` vs `Adjustment::Unadjusted` trong DB và trên UI
- Trong schema SQLite (`storage.rs:78-115`), khóa chính bảng `bars` là `(exchange, symbol, interval, adjustment, ts)` — kiến trúc DB đã thiết kế sẵn sàng để lưu song song cả `'splits'`, `'dividends'` và `'unadjusted'`.
- Tuy nhiên ở tầng ứng dụng: **`16 / 17` vị trí gọi dữ liệu trong `crates/sumi-desktop/src/commands/` khóa cứng `Adjustment::Splits`**; `ui/src/api/tauri.ts:63, 74` mặc định `'splits'`; và **trên toàn bộ giao diện React hoàn toàn không có nút `ADJ` để chuyển đổi giữa Giá điều chỉnh và Giá gốc chưa điều chỉnh**.

---

## 1.3. Thống Kê Thực Chứng 100% Từ `sumi_market.db` Đang Chạy Thật

Truy vấn chỉ đọc (`?mode=ro`) trực tiếp trên `~/Library/Application Support/com.sumi.supercharts/sumi_market.db`:

### A. Quy mô tổng thể Cơ sở dữ liệu
- **Dung lượng tệp trên đĩa:**
  - `sumi_market.db`: **`88.502.272` bytes (`84,40 MiB`)** (kèm `-wal`: `82.432` bytes, `-shm`: `32.768` bytes, và bản sao lưu `sumi_market.db.v1.bak`: `88.403.968` bytes).
  - `sumi_workspace.db`: **`77.824` bytes (`76 KiB`)** (kèm `-wal`: `243.136` bytes — lưu `5` watchlists, `4` indicator templates, cài đặt bố cục, hình vẽ, cảnh báo và sổ lệnh Paper Broker).
  - *(Ghi chú thêm: Trong thư mục mã nguồn còn có bản cache phát triển `/Users/mizuhara/workspace/sumi/data/cache/sumi_market.db` nặng `86 MiB` chứa `473` chuỗi VN `1D` tách biệt với App Support DB).*
- **Tổng số bản ghi trong App Support `sumi_market.db`:**
  - Tổng số mã định danh (`symbols`): **`486` mã**
  - Tổng số chuỗi thời gian (`series_meta`): **`492` chuỗi**
  - Tổng số cây nến (`bars`): **`958.282` nến** (`0` nến mồ côi, `100%` đều mang `adjustment = 'splits'`)
- **Phân bổ `958.282` nến theo nguồn gốc (`bars.source`):**
  - `cafef_adjusted`: **`919.481` nến** (`95,95%`)
  - `cafef_spliced`: **`16.678` nến** (`1,74%` — thuộc `9` mã HOSE đã từng mở online và kích hoạt ghép nối tỷ lệ)
  - `tradingview`: **`11.647` nến** (`1,22%`)
  - `cafef_index`: **`10.471` nến** (`1,09%` — lịch sử dài hạn `VNINDEX` & `HNXINDEX`)
  - `tradingview_live`: **`5` nến** (`< 0,01%`)

### B. Bảng phân rã chi tiết theo `Sàn × Loại tài sản × Khung giờ × Adjustment`

| Sàn (`Exchange`) | Loại (`market_type`) | Khung giờ (`interval`) | Chế độ (`adjustment`) | Số mã | Tổng số nến | TB nến / mã | Ngày cũ nhất (`UTC`) | Ngày mới nhất (`UTC`) | Ghi chú thực chứng |
|---|---|---:|---|---:|---:|---:|---|---|---|
| **HOSE** | `stock` | `1D` | `splits` | **264** | **567.768** | 2.150,6 | `2016-01-04` | `2026-11-23` | `17` mã mới tới `20–23/11`, `247` mã dừng ở `06/11` |
| **HOSE** | `index` | `1D` | `splits` | **2** | **6.691** | 3.345,5 | `2000-07-28` | `2026-11-23` | `VNINDEX` (`6.391` nến từ `2000`), `VN30` (`300` nến) |
| **HOSE** | `stock` | `1W` | `splits` | **1** | **601** | 601,0 | `2015-04-20` | `2026-11-23` | Chỉ duy nhất `HOSE:FPT` |
| **HOSE** | `stock` | `4h` | `splits` | **2** | **1.100** | 550,0 | `2025-09-10` | `2026-11-20` | Chỉ `HOSE:FPT` (`600`) & `HOSE:VCB` (`500`) |
| **HOSE** | `stock` | `1h` | `splits` | **2** | **1.100** | 550,0 | `2026-06-03` | `2026-11-20` | Chỉ `HOSE:FPT` (`600`) & `HOSE:VCB` (`500`) |
| **HOSE** | `stock` | `1m` | `splits` | **1** | **500** | 500,0 | `2026-11-18` | `2026-11-20` | Chỉ duy nhất `HOSE:FPT` (~3 phiên) |
| **HNX** | `stock` | `1D` | `splits` | **90** | **196.673** | 2.185,3 | `2016-01-04` | `2026-11-06` | Toàn bộ `90/90` mã dừng ở `2026-11-06` |
| **HNX** | `index` | `1D` | `splits` | **1** | **4.681** | 4.681,0 | `2005-11-02` | `2026-11-20` | `HNX:HNXINDEX` (21 năm lịch sử) |
| **HNX** | `futures` | `1D` | `splits` | **1** | **300** | 300,0 | `2025-09-10` | `2026-11-20` | `HNX:VN30F1!` (chưa có intraday trong DB) |
| **UPCOM** | `stock` | `1D` | `splits` | **117** | **175.568** | 1.500,6 | `2016-01-04` | `2026-11-06` | Toàn bộ `117/117` mã dừng ở `2026-11-06` |
| **BINANCE** | `spot` | `1D` | `splits` | **5** | **1.500** | 300,0 | `2026-01-27` | `2026-11-22` | `BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT` |
| **NASDAQ** | `stock` | `1D` | `splits` | **3** | **900** | 300,0 | `2025-09-15` | `2026-11-20` | `AAPL, MSFT, NVDA` |
| **AMEX** | `fund` | `1D` | `splits` | **1** | **300** | 300,0 | `2025-09-15` | `2026-11-20` | `SPY` |
| **FX_IDC** | `forex` | `1D` | `splits` | **1** | **300** | 300,0 | `2025-09-28` | `2026-11-19` | `USDVND` |
| **OANDA** | `commodity` | `1D` | `splits` | **1** | **300** | 300,0 | `2025-09-23` | `2026-11-19` | `XAUUSD` |
| **TỔNG CỘNG** | — | — | `splits` | **486 mã (492 chuỗi)** | **958.282** | — | `2000-07-28` | `2026-11-23` | `0` nến ở khung `5m`, `15m`, `30m`, `1M` |

### C. Ba sự thật quan trọng rút ra từ CSDL thực tế

1. **Độ phủ Intraday & Độ phủ Mã toàn thị trường VN:**
   - Trong `471` cổ phiếu Việt Nam có trong DB (`264 HOSE + 90 HNX + 117 UPCOM`), có tới **`469` cổ phiếu (`99,58%`) CHỈ CÓ KHUNG `1D`**, hoàn toàn chưa có bất kỳ nến intraday (`1h / 15m / 5m / 1m`) nào được lưu sẵn (chỉ tải theo nhu cầu `on-demand` khi người dùng bấm mở đúng mã + khung đó trên biểu đồ).
   - Toàn thị trường chứng khoán Việt Nam có hơn `1.600` mã trên 3 sàn, nhưng DB pre-seed chỉ chứa **`471` mã thanh khoản lọc sẵn** từ gói dữ liệu CafeF QMV (`CafeF.SolieuGD.Adjusted.Upto06112026.csv`). Khoảng `~1.100+` mã nhỏ/UPCOM khác chưa tồn tại trong `sumi_market.db` (dù vẫn tìm và mở lẻ từng mã được khi có Internet qua TradingView).
2. **"Bẫy Dữ Liệu EOD Cũ" (`Stale EOD Trap`) trong `Stock Screener`:**
   - Có tới **`435 / 475` chuỗi VN `1D` (`91,6%`)** đang có cây nến cuối cùng dừng ở ngày **`2026-11-06`** (ngày xuất file CSV CafeF), trong khi chỉ `19` mã từng được người dùng bấm xem trực tiếp mới cập nhật lên `2026-11-20..23`.
   - Nguyên nhân nằm ở `crates/sumi-desktop/src/commands/scanner.rs:277-309`: khi chạy `run_stock_screener`, hàm kiểm tra `cached_count < 15` mới gọi mạng tải dữ liệu mới; vì cả `435` mã CafeF đều đã có `~1.500–2.500` nến (`>= 15`), **Screener bỏ qua hoàn toàn bước cập nhật online** và âm thầm chấm điểm 91,6% cổ phiếu trên nến cũ cách đây gần 3 tuần!
3. **Cơ chế khung `1W` và `1M` (Tải trực tiếp vs Tự tổng hợp):**
   - Khi xem biểu đồ bình thường, `1W` và `1M` được **tải trực tiếp thành chuỗi độc lập từ TradingView** (`cache_service.rs` / `tradingview_feed.rs`), **KHÔNG tự tổng hợp (`rollup`) từ nến `1D` CafeF trong SQLite**. Hệ quả: nếu bật chế độ Offline trên một mã chưa từng mở khung `1W`/`1M` lúc Online, biểu đồ `1W`/`1M` sẽ trống rỗng dù máy đang có sẵn 10 năm nến `1D`!
   - Chỉ riêng khi bật **`Causal Bar Replay`** (`crates/sumi-desktop/src/commands/replay.rs:106-118, 210-242`), hệ thống mới tự động tổng hợp nhân quả `1W` và `1M` từ nến cơ sở (`1D` hoặc intraday).

---

## 1.4. Nguồn 3 — Kiểm Tra Liên Kết Yumeko & Các Dữ Liệu Thị Trường VN Sumi CHƯA THỂ LẤY

### A. Kiểm chứng cầu nối `yumeko-sumi-contract`
- Kiểm tra trực tiếp `/Users/mizuhara/workspace/yumeko/crates/yumeko-sumi-contract/src/lib.rs` và quét toàn bộ cây mã nguồn của cả hai workspace `/Users/mizuhara/workspace/sumi` và `/Users/mizuhara/workspace/yumeko`:
  - `yumeko-sumi-contract` định nghĩa schema trao đổi (`SUMI_CONTRACT_VERSION_V1`, snapshot chỉ báo/chế độ thị trường) nhưng hiện **chỉ được import một chiều bên trong nội bộ Yumeko** (`yumeko-app-service`, `yumeko-source-dnse`, `yumeko-source-qmv`).
  - Toàn bộ `Cargo.toml` và mã nguồn của **Sumi (`sumi-data`, `sumi-core`, `sumi-desktop`, `ui`) hoàn toàn KHÔNG tham chiếu tới `yumeko-sumi-contract`, KHÔNG mở kết nối sang DB của Yumeko, và KHÔNG có kênh IPC/HTTP nào nối sang Yumeko hay DNSE Lightspeed**. Hai hệ thống hiện vận hành độc lập 100%.

### B. Danh mục thẳng thắn các dữ liệu thị trường Việt Nam mà Sumi HIỆN CHƯA CÓ NGUỒN LẤY

| Nhóm dữ liệu thiếu | Tình trạng hiện tại trong Sumi `v2.2.0` | Hệ quả đối với phân tích thực chiến | Nguồn khả dĩ để bổ sung |
|---|---|---|---|
| **1. Khớp lệnh từng tick (`Time & Sales`)** | **Chưa có (0%).** Chỉ có nến gộp tối thiểu `1m` (trễ 15p trên TV). | Không xem được sổ khớp lệnh từng lô lớn ("cá mập ăn hàng"), không dựng được Footprint thật. | DNSE Lightspeed MQTT (`yumeko-source-dnse`), SSI FastConnect, hoặc Vietstock/FireAnt. |
| **2. Mua / Bán chủ động thực tế (`Active Buy / Sell Volume`)** | **Chưa có dữ liệu thật.** Đang phải dùng công thức xấp xỉ hình học nến OHLCV (`QMV_FLOW`, `CVD`, `intrabar_buy_fraction`). | Dễ bị nhiễu ở các phiên kéo xả ATC hoặc nến rút chân giả tạo thanh khoản. | DNSE MQTT tick stream hoặc EOD Active Buy/Sell từ nguồn nội địa. |
| **3. Sổ lệnh 3 / 10 bước giá (`Order Book Level 2 / DOM thật`)** | **Chưa có (0%).** Thậm chí `bid`/`ask` Level 1 từ TradingView cũng bị gọt bỏ ở `commands/market.rs:134-145` (và trễ 15p). | Không nhìn được độ dày kê lệnh Mua/Bán, dư Mua trần / dư Bán sàn trực tiếp. | DNSE Lightspeed Top-3/Top-10 Offer/Bid MQTT. |
| **4. Giao dịch Khối ngoại, Tự doanh & Thỏa thuận** | **Chưa có (0%).** | Thiếu mất một trụ cột quan trọng của TTCK Việt Nam (mua ròng/bán ròng khối ngoại, tự doanh CTCK, sang tay thỏa thuận). | Cầu nối sang kho dữ liệu Yumeko / DNSE / CafeF API. |
| **5. Lịch sự kiện Doanh nghiệp (`GDKHQ, Cổ tức, Chia tách, ĐHCĐ, Ngày ra BCTC`)** | **Chưa có trên trục biểu đồ.** | Người dùng không thấy mốc `D` (Cổ tức), `S` (Chia tách), `E` (BCTC) dưới chân nến để giải thích các khoảng trống giá (`Gap`) hoặc tránh ngày chốt quyền. | Có thể lấy ngay một phần từ `tradingview-rs` (`SymbolInfo.earnings_release_date` + `news::CorpActivitiesAll`) hoặc CafeF/Vietstock. |
| **6. Chỉ số Tài chính Doanh nghiệp (`P/E, P/B, EPS, ROE, Doanh thu, LNST`)** | **Chưa nối lên Sumi (0% trong DB/UI).** | Không lọc được cổ phiếu cơ bản tốt kết hợp PTKT (`CANSLIM / QMV Fundamental`), không vẽ được lịch sử định giá `P/E, P/B`. | **Mở khóa ngay lập tức từ module `fundamental` (`1.340+` chỉ tiêu) có sẵn trong `tradingview-rs`!** |
| **7. Độ rộng thị trường toàn sàn đầy đủ (`~1.600` mã + Số mã Trần/Sàn)** | `sumi-core` có sẵn hàm tính Breadth (`breadth.rs`), nhưng chưa nối lên UI và DB mới có `471` mã (bị cũ ngày `06/11`). | Thiếu bức tranh tổng thể dòng tiền lan tỏa hay "xanh vỏ đỏ lòng" mỗi ngày. | Đồng bộ đủ mã qua `tradingview-rs::list_symbols` + nối `breadth.rs` lên UI. |
| **8. Vi cấu trúc Phái sinh (`Open Interest - OI`, Khối lượng ngoại F1..F4, Độ lệch `Basis`)** | Mới chỉ vẽ nến `HNX:VN30F1!` đơn lẻ, chưa có khối lượng mở (`OI`) qua đêm và chưa tính tự động đường `Basis = VN30F1! - VN30`. | Thiếu chỉ báo tâm lý phòng vệ/đầu cơ qua đêm của nhà tạo lập trên thị trường phái sinh. | Ghép `HNX:VN30F1! - HOSE:VN30` ngay từ dữ liệu sẵn có (cho `Basis`); `OI` cần nguồn HNX/DNSE. |

---

# CHƯƠNG 2 — BẢN ĐỒ TOÀN BỘ TÍNH NĂNG HIỆN TẠI CỦA SUMI `v2.2.0` (`CODEBASE & UI FEATURE INVENTORY`)

Quy ước nhãn trạng thái kiểm toán:
- `✅ [Hoàn chỉnh trên UI]` — Đã nối đầy đủ và chạy thật trên giao diện người dùng.
- `⚠️ [Có trong Rust / Ẩn hoặc Một phần trên UI]` — Đã lập trình trong `sumi-core` / `sumi-data` / `sumi-desktop` nhưng chưa nối lên giao diện React, bị ẩn, khóa cứng tham số hoặc vướng lỗi kết nối.
- `🧪 [Mô phỏng / Xấp xỉ]` — Vận hành dựa trên giả định hoặc công thức xấp xỉ toán học từ nến OHLCV (không có dữ liệu tick/sổ lệnh thật).

---

## 2.1. Phân hệ 1 — Biểu đồ Supercharts, Đa khung giờ & Quy tắc hiển thị Việt Nam

| Tính năng | Trạng thái | Chi tiết thực tế & Giới hạn kiểm chứng từ mã nguồn | Tham chiếu `file:line` |
|---|---|---|---|
| **Động cơ dựng biểu đồ (`Chart Engine`)** | `✅ [Hoàn chỉnh trên UI]` | Dùng **`klinecharts@^10.1.1`** (Canvas HTML5 hiệu năng cao). *Lưu ý:* Toàn bộ module toán học khung nhìn `crates/sumi-core/src/viewport/` (`10.359` dòng từ bản `v1.x` egui) hiện không còn được UI React sử dụng. | `ui/package.json:16`<br>`ui/src/chart/KLineViewport.tsx:581-617` |
| **6 Kiểu biểu đồ (`Chart Styles`)** | `✅ [Hoàn chỉnh trên UI]` | Hỗ trợ 6 kiểu: Nến đặc (`candle_solid`), Nến rỗng (`candle_stroke`), **`Heikin-Ashi`** (biến đổi nhân quả khóa `prevClosedHaBarRef` chống trôi giá mở khi nến live nhảy), Thanh `OHLC`, Đường (`line`), Vùng (`area`). **Chưa có:** `Baseline`, `Renko`, `Kagi`, `Line Break`, `Point & Figure`, `Range`, `Hollow Volume Candles`. | `ui/src/types/market.ts:5`<br>`ui/src/chart/heikinAshi.ts:7-32`<br>`ui/src/components/TopHeader.tsx:34-41` |
| **Thang giá (`Linear / % / Log` & `Auto`)** | `⚠️ [Một phần trên UI]` | 3 nút `TĐ` (`normal`), `%` (`percentage`), `Log` (`logarithm`) chạy hoàn chỉnh, lưu riêng từng ô. Kéo dọc trục Y thu phóng tự do, nhấp đúp vùng 78px trục Y bên phải hoặc `Alt+R` để Auto-fit. **Thiếu:** Nút bật/tắt trạng thái `Auto` tường minh dưới đáy trục giá và chế độ **`Lock Price-to-Bar Ratio`**. | `ui/src/components/TopHeader.tsx:380-393`<br>`ui/src/chart/KLineViewport.tsx:439-448, 978-985` |
| **Múi giờ (`Timezone`)** | `⚠️ [Một phần trên UI]` | Nút địa cầu xoay vòng `Auto -> UTC+7 (Asia/Ho_Chi_Minh) -> UTC`. **Điểm khuyết:** Chế độ `Auto` gán `UTC+7` cho mã VN nhưng **ép cứng `Etc/UTC` cho mọi mã quốc tế** (cổ phiếu Mỹ `NASDAQ:AAPL` hiện giờ UTC thay vì `America/New_York`, bỏ qua trường `ResolvedSymbol.timezone`). | `ui/src/components/TopHeader.tsx:623-646`<br>`ui/src/chart/KLineViewport.tsx:51-55` |
| **Vạch ngăn phiên giao dịch VN (`ICT Session Breaks`)** | `✅ [Hoàn chỉnh trên UI]` | Nút `"Phiên"` vẽ vạch dọc đứt nét phân tách **giờ giao dịch sàn Việt Nam (`UTC+7` Indochina Time)**: `09:00` mở cửa sáng, `13:00 Chiều` sau nghỉ trưa, và `14:30 ATC` trên khung `1m, 5m, 15m, 1h` (tối đa 45 vạch gần nhất). *Lưu ý:* Bỏ sót khung `30m` (`vnMarket.ts:84`), và đây là vạch phiên giao dịch VN chứ không phải hộp phiên London/NY Killzone của trường phái SMC/Forex. | `ui/src/utils/vnMarket.ts:79-120`<br>`ui/src/chart/overlays/tradingViewOverlays.ts:515-583` |
| **Dải Trần / Tham chiếu / Sàn (`T/TC/S`) & Đếm ngược nến** | `✅ / 🧪 [Xấp xỉ giá TC từ Close trước]` | Tự động tính chuẩn biên độ làm tròn bước giá trong Rust (`HOSE ±7%` bậc thang `10/50/100`, `HNX ±10%` bước `100`, `UPCOM ±15%` bước `100`, ẩn trên `VNINDEX/VN30F`), vẽ 3 đường Tím/Vàng/Xanh lơ + thẻ tóm tắt góc phải trên; đồng hồ đếm ngược đóng nến cập nhật mỗi `1s`. **Lưu ý xấp xỉ:** Hệ thống gán `reference_price = Close phiên trước`, trong khi thực tế trên sàn **UPCOM** giá tham chiếu pháp định là **bình quân gia quyền (`VWAP`) phiên trước** (không phải giá đóng cửa!), và trên HOSE/HNX vào ngày GDKHQ hoặc ETF/CW (bước giá `10 VND`) sẽ có sai số nhỏ. Ngoài ra khung `4h` căn theo `00:00/04:00 UTC` (`07:00/11:00 ICT`) nên tách phiên sáng VN (`09:00–11:30`) làm hai thanh `4h`. | `crates/sumi-desktop/src/commands/market.rs:242-267`<br>`crates/sumi-core/src/data/timeframe.rs:135-147` |
| **Lưới đa biểu đồ (`1..4` ô) & Lưu bố cục (`Layouts`)** | `✅ [Hoàn chỉnh trên UI]` | 5 bố cục: `1`, `2v` (2 cột), `2h` (2 hàng), `3` (1 trái lớn + 2 phải), `4` (`2×2`), lưu/nạp/xóa bố cục đặt tên trong SQLite `chart_layouts`. Mặc định mở đa khung `1D / 4h / 1h / 15m`. | `ui/src/App.tsx:212-264`<br>`ui/src/store/workspaceStore.ts:65-146, 1102-1129` |
| **Cửa sổ phụ tách rời (`Detached Sub-Windows`)** | `⚠️ [Có lỗi quyền ACL & Đụng khóa Stream]` | Nút `ExternalLink` gọi `open_detached_window` tạo cửa sổ OS riêng (`?detached=1&ticker=...`). **Phát hiện 2 lỗi kỹ thuật nghiêm trọng:**<br>1. `crates/sumi-desktop/capabilities/default.json:5` chỉ cấp quyền `"windows": ["main"]`, nên cửa sổ phụ `chart_<timestamp>` bị Tauri v2 chặn quyền lắng nghe sự kiện đồng bộ (`event:allow-listen`)!<br>2. Cửa sổ phụ cũng khởi tạo `slotId = "pane-0"`, dùng chung tên khóa `stream_pane-0` trong `AppState.bar_streams` (`KLineViewport.tsx:626` -> `market.rs:845`), làm **ngắt luồng nến trực tiếp của ô số 1 trên cửa sổ chính**! | `crates/sumi-desktop/src/commands/workspace.rs:638-664`<br>`crates/sumi-desktop/capabilities/default.json:5`<br>`ui/src/chart/KLineViewport.tsx:626` |
| **5 Chế độ Đồng bộ đa ô (`Mã / Khung / Tâm / Cuộn / Hình vẽ`)** | `⚠️ [Một phần trên UI]` | - `Mã` (`symbol`), `Khung` (`interval`), `Tâm` (`crosshair` RAF 60fps), `Hình vẽ` (`drawings`): hoạt động mượt.<br>- `Cuộn` (`dateRange`): `TimeRangeBus` chỉ đồng bộ mốc thời gian tâm màn hình (`scrollToTimestamp`), **chưa đồng bộ độ phóng to/thu nhỏ (`barSpace`)** và chưa đồng bộ sang cửa sổ phụ. | `ui/src/chart/crosshairBus.ts:20-165`<br>`ui/src/components/TopHeader.tsx:490-508` |
| **Cây Đối Tượng (`Object Tree`)** | `✅ [Hoàn chỉnh trên UI]` | Tab `"Cây đối tượng"` quản lý toàn bộ Chỉ báo (ẩn/hiện, cài đặt, xóa) và Hình vẽ (đặt cảnh báo 1-click, ẩn/hiện, khóa, xóa, lọc hiển thị theo khung giờ). *Thiếu:* Bấm vào dòng hình vẽ chưa cuộn biểu đồ tới hình đó, chưa có thư mục gom nhóm (`Folders`) hay đổi tên hình vẽ. | `ui/src/components/RightWidgetBar.tsx:934-1133` |
| **Hệ thống Phím tắt & Xuất ảnh `PNG`** | `✅ [Hoàn chỉnh trên UI]` | Phím tắt `Cmd/Ctrl+K` hoặc gõ thẳng chữ cái để mở tìm mã, `Alt+T/H/J/V/F`, `Shift+M` (thước đo), `Alt+R` (Reset view), `Cmd+Z / Cmd+Shift+Z` (Undo/Redo hình vẽ), `Delete`, `Space` (Play/Pause Replay). Nút máy ảnh xuất ảnh `.png` tức thì. | `ui/src/App.tsx:121-189`<br>`ui/src/chart/KLineViewport.tsx:1539-1625` |
| **Chuyển đổi Giá điều chỉnh (`ADJ`)** | `⚠️ [Có trong DB schema / Ẩn hoàn toàn trên UI]` | Cột `adjustment` có sẵn trong SQLite và `get_bars`, nhưng UI khóa cứng `'splits'`, không có nút bật/tắt `ADJ`. | `ui/src/api/tauri.ts:63, 74` |

---

## 2.2. Phân hệ 2 — Chỉ báo Kỹ thuật & Mẫu Chỉ báo (`Indicators & Templates`)

### A. 16 Chỉ báo đang mở trên Giao diện (`ui/src/chart/indicators/catalog.ts`)
Giao diện chia làm 2 tầng rõ ràng (**11 Chỉ báo QMV Cốt lõi** mở sẵn + **5 Chỉ báo Định lượng Phụ trợ** thu gọn):

| # | Mã | Tên tiếng Việt trên UI | Tầng | Khung vẽ | Nơi tính toán thực tế | Trạng thái |
|---:|---|---|---|---|---|---|
| 1 | **`AFC`** | Nón Xác Suất Tương Đồng Lịch Sử (`P10..P90`, `H=10`, `Top-K=100`, chiếu biên độ Trần/Sàn VN) | QMV Cốt lõi | Main | **Rust (`sumi-core` `analog_cone.rs`)** qua `compute_analog_cone` | `✅ [Hoàn chỉnh trên UI]` |
| 2 | **`MA`** | Đường Trung Bình Động Chuẩn QMV (`20, 50, 100, 200`) | QMV Cốt lõi | Main | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |
| 3 | **`ICHIMOKU`** | Mây Ichimoku Chuẩn QMV V7 (`6, 18, 36`, dịch `18` nến tương lai) | QMV Cốt lõi | Main | JS (`customIndicators.ts:169-310`) | `✅ [Hoàn chỉnh trên UI]` |
| 4 | **`BOLL`** | Dải Bollinger Bands (`20, 2`) | QMV Cốt lõi | Main | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |
| 5 | **`EMA`** | Trung Bình Lũy Thừa (`20, 50, 200`) | QMV Cốt lõi | Main | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |
| 6 | **`VOL`** | Khối Lượng & Thanh Khoản (`MA5, MA20`) | QMV Cốt lõi | Sub | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |
| 7 | **`QMV_FLOW`** | Chu Kỳ Dòng Tiền `T+ / T++ / Sóng` (`Z-50` chuẩn hóa vị trí đóng cửa × RVOL) | QMV Cốt lõi | Sub | JS (`customIndicators.ts:523-610`) — gắn nhãn trung thực `OHLCV Proxy` | `🧪 [Xấp xỉ từ OHLCV]` |
| 8 | **`RS_VNINDEX`** | Sức Mạnh Tương Đối so với `HOSE:VNINDEX` + đường MA(`20`) | QMV Cốt lõi | Sub | Hybrid: Tải `VNINDEX` từ Rust SQLite + chia tỷ số trong JS (`:615-668`) | `✅ [Hoàn chỉnh trên UI]` |
| 9 | **`CCI`** | Commodity Channel Index Chuẩn QMV (`14`, vạch `+100/0/-100`) | QMV Cốt lõi | Sub | JS (`customIndicators.ts:942-997`) | `✅ [Hoàn chỉnh trên UI]` |
| 10 | **`RSI`** | Wilder RSI (`14`, vạch `70/50/30` — đã sửa lỗi SMA RSI mặc định của KLineChart) | QMV Cốt lõi | Sub | JS (`customIndicators.ts:878-936`) | `✅ [Hoàn chỉnh trên UI]` |
| 11 | **`MACD`** | MACD Chuẩn Quốc tế `1x (DIF - DEA)` (`12, 26, 9` — đã sửa hệ số `2x` kiểu TQ của KLineChart) | QMV Cốt lõi | Sub | JS (`customIndicators.ts:825-872`) | `✅ [Hoàn chỉnh trên UI]` |
| 12 | **`ATR`** | Biên Độ Dao Động Thực Wilder (`14`) | Phụ trợ | Sub | JS (`customIndicators.ts:471-517`) | `✅ [Hoàn chỉnh trên UI]` |
| 13 | **`VRVP`** | Hồ Sơ Khối Lượng Vùng Nhìn Thấy (`24` mức giá + đường `POC`) | Phụ trợ | Main | JS (`customIndicators.ts:674-765`) — phân bổ đều theo dải nến, thiếu `VAH/VAL` & tách Vol Mua/Bán | `🧪 [Xấp xỉ / Rút gọn]` |
| 14 | **`VWAP`** | VWAP Phiên & Dải Độ Lệch Chuẩn (`±1σ, ±2σ`, tự reset đầu ngày) | Phụ trợ | Main | JS (`customIndicators.ts:386-465`) | `✅ [Hoàn chỉnh trên UI]` |
| 15 | **`OBV`** | Khối Lượng Cân Bằng (`OBV` + `MA20`) | Phụ trợ | Sub | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |
| 16 | **`DMI`** | Chỉ Số Định Hướng Xu Hướng (`+DI, -DI, ADX, ADXR`) | Phụ trợ | Sub | JS (KLineChart built-in) | `✅ [Hoàn chỉnh trên UI]` |

### B. Các Chỉ báo ĐÃ VIẾT SẴN trong `sumi-core::indicators` nhưng CHƯA NỐI lên Biểu đồ UI!
Đây là phát hiện rất đáng chú ý: Lõi Rust `crates/sumi-core/src/indicators/` đang sở hữu nhiều bộ tính chỉ báo chuyên sâu mà giao diện biểu đồ hoàn toàn chưa mở:
1. `⚠️` **`SessionMarketProfile` — Biểu đồ TPO / Market Profile (`crates/sumi-core/src/indicators/market_profile.rs:1-420`):** Tính đầy đủ chữ cái TPO theo phiên, `POC`, `Value Area High (VAH)`, `Value Area Low (VAL)` `70%`, `Initial Balance (IB)`, và kéo dài `Naked POC`!
2. `⚠️` **`compute_anchored_vwap` — Anchored VWAP neo tại nến bất kỳ (`crates/sumi-core/src/indicators/vwap.rs:212-335`):** Đã viết hoàn chỉnh kèm dải độ lệch chuẩn `±1σ, ±2σ, ±3σ`, chưa nối lên công cụ vẽ/chỉ báo UI!
3. `⚠️` **`compute_volume_profile` đầy đủ `VAH / VAL / Up-Down Volume` (`crates/sumi-core/src/indicators/volume_profile.rs:109-351`):** Bản Rust tính đầy đủ vùng giá trị `70%` (`VAH`, `VAL`) và tách khối lượng Mua/Bán ước lượng (`intrabar_buy_fraction`), trong khi bản JS `VRVP` trên UI mới chỉ vẽ cột tổng khối lượng đơn sắc + 1 đường `POC`.
4. `⚠️ 🧪` **`compute_cvd` — Cumulative Volume Delta (`crates/sumi-core/src/indicators/cvd.rs:1-265`):** Tính `CVD` lũy kế hoặc reset theo ngày kèm đường `SMA` (xấp xỉ từ OHLCV), chưa đưa lên UI.
5. `⚠️` **5 Chỉ báo Kỹ thuật Mở rộng trong `crates/sumi-core/src/indicators/extended.rs`:**
   - **`SuperTrend`** (`compute_supertrend`, `:120-215`)
   - **`Parabolic SAR (`PSAR`)`** (`compute_psar`, `:235-355`)
   - **`Keltner Channels`** (`compute_keltner_channels`, `:375-435`)
   - **`Stochastic Oscillator (`%K, %D`)`** (`compute_stochastic`, `:455-535`)
   - **`Money Flow Index (`MFI`)`** (`compute_mfi`, `:555-625`)
   *(Hiện 5 chỉ báo này chỉ được gọi ngầm bên trong bộ quét `scanner/catalog.rs`, đã bị gỡ khỏi `INDICATOR_CATALOG` trên biểu đồ; thậm chí trong `sumi_workspace.db` cũ vẫn còn lưu mẫu `tpl_vn30f_scalp` tham chiếu tới `SuperTrend` không còn hiển thị được!).*
6. `⚠️` **Nhận diện Mẫu hình Nến Nhật (`crates/sumi-core/src/indicators/patterns.rs:1-551`):** Nhận diện tự động `Doji, Hammer, Shooting Star, Bullish/Bearish Engulfing, Morning/Evening Star, Piercing, Dark Cloud Cover, Three White Soldiers, Three Black Crows, Marubozu, Harami` — hiện chỉ dùng trong `Screener`, chưa có lớp đánh dấu trực tiếp trên biểu đồ nến.

### C. Tùy biến Tham số / Màu sắc & Mẫu Chỉ báo (`Indicator Templates`)
- `✅ [Hoàn chỉnh trên UI — có điểm khuyết nhỏ]` Hộp thoại `IndicatorSettingsModal.tsx` chỉnh tham số số học, màu từng đường, độ dày nét (`1..3px`) và lưu SQLite; menu `"Mẫu"` (`TopHeader.tsx:277-357`) lưu/nạp/xóa mẫu chỉ báo kèm 4 mẫu QMV cài sẵn.
- **Điểm khuyết nhỏ:** Khi bấm lưu trong `IndicatorSettingsModal.tsx:122-127`, code gán cứng `style: 'solid'`, làm mất nét đứt mặc định của đường `Chikou` (`ICHIMOKU`) hoặc dải `±1σ/±2σ` (`VWAP`); riêng `VRVP` vẽ thủ công trong hàm `draw()` nên đổi màu trong modal không có tác dụng.

---

## 2.3. Phân hệ 3 — Công cụ Vẽ Phân Tích Kỹ Thuật (`37 Drawing Tools`)

- `✅ [Hoàn chỉnh trên UI]` Đủ chính xác **37 công cụ vẽ người dùng** chia thành **8 nhóm** trên thanh công cụ dọc bên trái (`ui/src/chart/drawingIcons.tsx:26-122`, `LeftDrawingRail.tsx`, `proOverlays.ts`, `tradingViewOverlays.ts`):
  1. **Đường xu hướng & Ngang/Dọc (`11`):** `segment` (`Alt+T`), `rayLine`, `straightLine`, `arrow`, `horizontalStraightLine` (`Alt+H`), `horizontalRayLine` (`Alt+J`), `horizontalSegment`, `verticalStraightLine` (`Alt+V`), `verticalRayLine`, `verticalSegment`, `priceLine`.
  2. **Kênh giá & Song song (`2`):** `priceChannelLine`, `parallelStraightLine`.
  3. **Fibonacci & Gann (`7`):** `fibonacciLine` (`Alt+F`), `fibonacciSegment`, `fibonacciExtension`, `fibonacciCircle`, `fibonacciSpiral`, `fibonacciSpeedResistanceFan`, `gannBox`.
  4. **Sóng Elliott & Mô hình Harmonic (`6`):** `xabcd`, `abcd`, `threeWaves` (`ABC`), `fiveWaves` (`1-2-3-4-5`), `eightWaves` (`8 sóng`), `anyWaves`.
  5. **Hình học & Vẽ tự do (`5`):** `rect`, `circle`, `triangle`, `parallelogram`, `brush` (cọ vẽ tự do).
  6. **Ghi chú & Nhãn giá (`2`):** `simpleAnnotation`, `simpleTag`.
  7. **Dự phóng R:R & Đường lệnh (`3`):** `longPosition` (Khung Mua R:R), `shortPosition` (Khung Bán R:R), `brokerOrderLine`.
  8. **Thước đo Giá & Thời gian (`1`):** `priceDateRange` (`Shift+M` — tính chênh lệch giá, `%`, số thanh nến, số ngày/giờ và tổng khối lượng).
- `✅` **Magnet (`Weak / Strong / Tắt`), Thanh công cụ nổi (`FloatingDrawingToolbar`), Khóa / Ẩn / Nhân bản (`Clone`) / Xóa, Undo / Redo (`30` bước `Cmd+Z / Cmd+Shift+Z`) & Lưu SQLite tự động:** Hoạt động đầy đủ (`KLineViewport.tsx:470-566, 1661-1701`).
- `⚠️ [3 Điểm khuyết thực tế phát hiện từ code]:`
  1. **Lỗi mất hình vẽ trên khung `30m`:** Hằng số `INTRADAY_INTERVALS` tại `ui/src/utils/vnMarket.ts:5` khai báo `['1m', '5m', '15m', '1h', '4h']` — **bỏ quên `'30m'`**, nên hình vẽ đặt chế độ hiển thị `"Intraday"` sẽ biến mất khi chuyển sang khung `30m`!
  2. Công cụ **`simpleAnnotation` (Ghi chú văn bản)** không có hộp thoại nhập nội dung chữ trên UI.
  3. `crates/sumi-core/src/drawings/geometry.rs:985-1161` & `crates/sumi-core/src/trading/portfolio.rs:199-315` thực chất đã cài đặt sẵn hình học **`FixedRangeVolumeProfile (FRVP)`**, **`AnchoredVwap (AVWAP)`** và hàm `extract_ticket_from_drawing` (bấm vào khung R:R `longPosition` / `Fibonacci` tự động điền Giá mua + `SL` + `TP` vào phiếu đặt lệnh Paper Broker) từ thế hệ Rust canvas cũ, nhưng **chưa được nối sang tầng KLineChart React UI**.

---

## 2.4. Phân hệ 4 — Hệ thống Cảnh báo (`Alerts`)

| Thành phần | Trạng thái | Hiện trạng thực tế (`UI` <-> `sumi-desktop` vs `sumi-core`) | Tham chiếu `file:line` |
|---|---|---|---|
| **Cảnh báo Giá, Đường vẽ (`6` loại) & `3` Chỉ báo (`RSI, MACD, EMA`)** | `✅ [Hoàn chỉnh trên UI]` | Tạo từ tab `"Cảnh báo"`, menu chuột phải trên biểu đồ, hoặc nút chuông `1-click` trên thanh công cụ hình vẽ / Cây đối tượng. Chạy cả khi nhận giá Live lẫn khi bấm từng nến trong `Bar Replay`. Có chuông tổng hợp Web Audio API (`C6 + E6`). | `ui/src/components/RightWidgetBar.tsx:656-930`<br>`crates/sumi-desktop/src/commands/workspace.rs:356-623` |
| **Bộ máy `AlertEngine` nâng cao trong `sumi-core`** | `⚠️ [Có sẵn trong Rust `sumi-core` / Bị bỏ qua ở Tauri & UI]` | `sumi-core::alerts::AlertEngine` đã viết sẵn: **(1) Cảnh báo trên bất kỳ tín hiệu nào trong `123` tín hiệu QMV (`SignalCatalogTrigger`)**, **(2) Cảnh báo điểm hợp lưu (`ConfluenceThreshold`)**, **(3) Chế độ lặp lại mỗi nến đóng (`OncePerBarClose` + `cooldown_bars`)**, **(4) Tự động kích hoạt lại khi tua lùi Replay**, và **(5) Nhật ký lịch sử nổ cảnh báo (`AlertTriggerEvent`)**. Nhưng `sumi-desktop/commands/workspace.rs` lại tự viết bộ kiểm tra đơn giản riêng (chỉ bắn 1 lần `one-shot`, không sửa được cảnh báo đã tạo, không có bảng Alert Log)! | `crates/sumi-core/src/alerts/engine.rs:270-384, 746`<br>`crates/sumi-desktop/src/commands/workspace.rs:582-623` |

---

## 2.5. Phân hệ 5 — Watchlist & Thông tin Chi tiết Mã

- `✅ [Hoàn chỉnh trên UI]` Quản lý nhiều danh mục theo dõi trong SQLite (khởi tạo sẵn 5 rổ: `QMV30`, `QMV60`, `Yumeko Core 10`, `DT1`, `VN Indices & Phái sinh`), thêm/xóa mã nhanh, cập nhật giá trực tiếp kèm hiệu ứng nháy màu chuẩn bảng điện VN (Tím Trần / Xanh / Vàng TC / Đỏ / Xanh lơ Sàn, giới hạn tần suất nháy `400ms` bảo vệ mắt); thẻ Chi tiết mã hiển thị Giá, `±%`, Trần / Tham chiếu / Sàn, Bước giá và Khối lượng (`RightWidgetBar.tsx:424-653`).
- `⚠️ [Điểm khuyết]:`
  1. Hàm xóa danh mục `delete_watchlist` đã có sẵn ở cả Rust (`workspace.rs:241`) và `ui/src/api/tauri.ts:282`, nhưng **trên giao diện không có nút Xóa hoặc Đổi tên Watchlist**.
  2. Chưa hỗ trợ kéo thả sắp xếp thứ tự mã, chưa bấm tiêu đề cột để sắp xếp theo `% Đổi`, chưa có Cờ màu (`Flags`) và bị lỗi lệch tên mã dây `HNX:VN301!` trên luồng quote trực tiếp như đã nêu ở Mục 1.1.B.

---

## 2.6. Phân hệ 6 — Bar Replay Nhân Quả (`Causal Bar Replay`)

- `✅ [Hoàn chỉnh trên UI — Điểm sáng kiến trúc của Sumi]`
  - **Cắt nến trực quan & Điều khiển đầy đủ:** Kéo rèm cắt trên biểu đồ hoặc menu chuột phải, tiến/lùi từng nến (`-1 / +1`), tự chạy ở tốc độ `0.5x .. 20x`, thanh trượt tiến độ (`ReplayControlBar.tsx`).
  - **Đồng bộ Đa Khung Thời Gian Nhân Quả (`Causal MTF Replay`):** Khi mở lưới `1h + 4h + 1D`, backend tự chọn khung mịn nhất (`1h`) làm nhịp cơ sở và tổng hợp nhân quả từng phần cho nến `4h` và `1D` — **cam kết tuyệt đối 0% rò rỉ giá đóng cửa buổi chiều khi mới đang ở nến buổi sáng** (`commands/replay.rs:85-208, 292-326`, đồng bộ luôn cả chỉ báo `AFC` theo `asOfTimestampMs`).
  - **Lưu & Khôi phục Phiên Luyện Tập (`Saved Replay Sessions`):** Lưu trọn vẹn mã, các khung giờ đang mở, vị trí con trỏ nến, chế độ Blind VÀ **toàn bộ trạng thái lệnh/vị thế của Sổ lệnh Sandbox `replay_broker`** vào SQLite (`replay.rs:733-1001`).
- `⚠️ [Giới hạn của `Blind Mode` & Module Huấn luyện ẩn]:`
  - Nút **`Blind Mode`** hiện mới là **"Chế độ Chống Lộ Kết Quả" (`Anti-Spoiler`)**: che giá hiện tại trên Watchlist/Thẻ chi tiết về đúng giá tại nến cắt và ẩn các hình vẽ tương lai (`RightWidgetBar.tsx:200-230`, `KLineViewport.tsx:1079-1094`), nhưng **CHƯA ẩn tên mã cổ phiếu (`HOSE:FPT`), chưa ẩn ngày tháng lịch sử và chưa nhân hệ số ẩn mức giá** (nên người tập vẫn nhớ được bối cảnh sóng lịch sử của mã quen thuộc).
  - Trong `sumi-core::trading::coaching` (`crates/sumi-core/src/trading/coaching.rs:1-3306`) đã viết sẵn nguyên bộ **Chấm điểm Phiên Luyện Tập & Báo cáo Tổng kết Cuối phiên (`SessionDebriefReport`, `PracticeDecisionLog`)** nhưng **chưa nối lên UI**.

---

## 2.7. Phân hệ 7 — Giao dịch Giấy (`Paper Broker`), Chart Trading & Nhật ký

| Hạng mục | Trạng thái | Chi tiết thực tế kiểm chứng từ mã nguồn | Tham chiếu `file:line` |
|---|---|---|---|
| **Quy tắc VSD `T+2` FIFO (`13:00 ICT`) & Thuế/Phí VN** | `✅ [Hoàn chỉnh trên UI]` | Vốn mặc định `500 triệu VND`, phí `0.15%`, thuế bán `0.10%`, lô chẵn `100` cp, cấm bán khống cơ sở, khóa cổ phiếu chờ về `T+2` đến đúng `13:00 ICT` ngày làm việc thứ 2 (có tính lịch nghỉ lễ VN `VIETNAM_HOLIDAYS_V1` trong `market_rules.rs`!), hiển thị chi tiết từng lô trong tab `"Lô FIFO VSD T+2"`. | `crates/sumi-desktop/src/commands/broker.rs:322-332, 490-495`<br>`ui/src/components/dock/PaperBrokerTab.tsx:735-803` |
| **Chế độ Phái sinh `HNX:VN30F1!`** | `✅ [Hoàn chỉnh trên UI]` | Tự động nhận diện `VN30F`: hệ số nhân `100.000 VND/điểm`, ký quỹ ban đầu chuẩn VSD **`17%`**, lô `1` HĐ, bước giá `0.1` điểm, mở khóa **Long & Short `T+0`**. | `crates/sumi-desktop/src/commands/broker.rs:333-346`<br>`ui/src/components/dock/PaperBrokerTab.tsx:541-557` |
| **Kéo thả trực tiếp `SL / TP / LO` trên biểu đồ (`Chart Trading`)** | `✅ [Hoàn chỉnh trên UI]` | Đặt lệnh nhanh từ menu chuột phải; hiển thị đường Vị thế kèm PnL trực tiếp; kéo thả mượt đường `SL`, `TP` và lệnh chờ trên biểu đồ tự động bắt dính bước giá sàn (`10/50/100 VND` hoặc `0.1 pt`). | `ui/src/chart/KLineViewport.tsx:1240-1400`<br>`ui/src/components/ChartContextMenu.tsx:181-246` |
| **Các loại lệnh (`LO / MP / ATO / ATC` vs `STOP`)** | `⚠️ / 🧪` | - `🧪` Khớp lệnh mô phỏng nhân quả từ nến OHLCV (`ATO` quy về `Market` khớp giá `Open`/`Close`, `ATC` khớp giá `Close`; lưu ý nhỏ trong `portfolio.rs:1939`: hàm `try_fill_pending_order_at` vẫn cộng `slippage_bps` cho cả lệnh `Limit` khiến lệnh Mua `LO` có thể khớp cao hơn nhẹ mức giá đặt nếu `slippage_bps > 0`).<br>- `⚠️` **Thiếu lệnh `STOP` trên phiếu đặt lệnh UI:** Lõi Rust `broker.rs:77, 709` và đường kéo trên biểu đồ đã hỗ trợ đầy đủ lệnh chờ `STOP` (mua vượt đỉnh / cắt lỗ tự động), nhưng mảng `ORDER_TYPES` ở `PaperBrokerTab.tsx:38` chỉ khai báo `['LO', 'MP', 'ATO', 'ATC']`, bỏ sót `'STOP'`! | `ui/src/components/dock/PaperBrokerTab.tsx:38`<br>`crates/sumi-core/src/trading/portfolio.rs:1939`<br>`crates/sumi-desktop/src/commands/broker.rs:709-715` |
| **Thang giá Đặt lệnh Nhanh (`Synthetic DOM / Footprint Ladder`)** | `⚠️ 🧪 [Có sẵn trong `sumi-core` / Chưa mở lên UI]` | Module `crates/sumi-core/src/trading/ladder.rs` (`1.776` dòng) đã cài đặt hoàn chỉnh thang giá DOM 21 bậc chuẩn bước giá HOSE/HNX/VN30F, gắn nhãn `CEIL/REF/FLR`, hiển thị khối lượng Mua/Bán ước lượng + `POC` và click đặt lệnh 1 chạm — **chưa nối lên Tauri/UI**. | `crates/sumi-core/src/trading/ladder.rs:1-1776` |
| **Nhật ký Giao dịch (`Trade Journal`) & Bộ Lập Kế Hoạch Vốn (`Trade Planner`)** | `⚠️ [Mở rất mỏng trên UI / Ẩn phần lớn trong Rust]` | - Trên UI (`PaperBrokerTab.tsx:805-946`): mới chỉ hiện bảng lệnh đã đóng kèm `MAE %`, `MFE %`, `R-multiple` và 1 ô ghi chú chữ đơn giản (thậm chí khi lưu ghi chú còn ghi đè chuỗi rỗng `''` làm mất `mistake_tag` ở `:213`!). Các chỉ số tổng hợp `Win Rate, Profit Factor, Expectancy, Equity Curve` trả về từ `broker.rs:500-516` cũng chưa được vẽ lên tab Paper Broker.<br>- Trong `sumi-core::journal` (`ledger.rs`, `planner.rs`): có sẵn **Bộ tính khối lượng đi vốn chuẩn Rủi ro (`compute_trade_plan` cảnh báo rủi ro Sàn cứng `T+2`)**, 7 thẻ `SetupTag`, 8 thẻ `MistakeTag` ("FOMO", "Dời SL"...), bảng kiểm `6` tiêu chí trước lệnh (`ChecklistState`), điểm `SQN` và thống kê thiệt hại do sai kỷ luật (`cost_of_mistakes`) — **hoàn toàn chưa nối lên UI**. | `ui/src/components/dock/PaperBrokerTab.tsx:213, 805-946`<br>`crates/sumi-core/src/journal/ledger.rs:18-200`<br>`crates/sumi-core/src/journal/planner.rs:24-200` |

---

## 2.8. Phân hệ 8 — Lọc Cổ Phiếu (`Stock Screener`) & Kiểm định Chiến lược (`Strategy Tester`)

### A. Stock Screener (`ui/src/components/dock/StockScreenerTab.tsx`, `crates/sumi-core/src/scanner/`)
- `✅ [Hoàn chỉnh trên UI]`
  - **Kho `123` Tín hiệu Định lượng & QMV (`SIGNAL_CATALOG` tại `catalog.rs`):** Gồm `114` tín hiệu Boolean, `8` tín hiệu Float, `1` Enum; phân thành 11 chuyên đề (`Technical: 24`, `Divergence: 19` [trễ nhân quả `3` nến xác nhận đáy/đỉnh pivot không vẽ lại!], `Flow Proxy: 17`, `Pattern: 15`, `Ichimoku V7: 13`, `Regime: 11`, `VSA: 10`, `Volume: 5`, `Health: 4`, `Support/Resistance: 3`, `Structure: 2`). Trong đó `33/123` tín hiệu dùng xấp xỉ dòng tiền từ OHLCV được gắn cờ trung thực `is_ohlcv_proxy = true`.
  - **Phân tầng Vũ trụ chuẩn QMV (`qmv_universe.rs`):** Lọc theo Rổ `QMV30`, `QMV60`, Phân lớp vốn hóa dòng tiền `DT1 / DT2 / DT3`, `3` Nhóm ngành lớn × `11` Ngành QMV (`Ngân hàng, Chứng khoán, BĐS, Thép...`), Phân khúc thị giá, Thanh khoản `KL20 >= 50k`, ghép nhiều chip tín hiệu `AND`, 5 nút lọc nhanh QMV và xuất CSV UTF-8 BOM.
- `⚠️ [4 Hạn chế / Tính năng ẩn trong Screener]:`
  1. **Khóa cứng khung `1D`** (`scanner.rs:402, 457`), chưa cho chọn lọc trên khung `1W`, `4h` hay `1h`, và dính **"Bẫy dữ liệu EOD cũ"** nêu tại Mục 1.3.C.
  2. Thiếu 2 tab danh mục `'pattern'` (15 mẫu nến) và `'structure'` trên thanh tab lọc nhanh (`StockScreenerTab.tsx:24-35`), và 3 biến state `minChangePct`, `macdFilter`, `minHealthScore` (`:92-94`) chưa có ô nhập trên giao diện.
  3. Chưa lưu được Bộ lọc cá nhân (`Saved Screens`) và chưa có nút đẩy thẳng danh sách kết quả sang một Watchlist mới.
  4. **Viên ngọc ẩn lớn nhất trong `sumi-core::scanner`: Module `breadth.rs` (`crates/sumi-core/src/scanner/breadth.rs:162-390`)** đã lập trình xong hoàn toàn **Độ rộng thị trường nội tại (`% mã > SMA20`, `% mã > EMA50`, Số mã Tăng/Giảm, Tỷ trọng dòng tiền VN30) VÀ Biểu đồ Xoay Dòng Tiền Tương Đối 4 Góc Phần Tư (`RRG`: `Leading / Weakening / Lagging / Improving` so với `VNINDEX`)** — nhưng **chưa được nối ra bất kỳ Tauri command hay màn hình UI nào**!

### B. Strategy Tester (`ui/src/components/dock/StrategyTesterTab.tsx`, `crates/sumi-core/src/backtest/`)
- `✅ [Hoàn chỉnh trên UI — Mạnh vượt trội ở cấp độ Định lượng]`
  - **12 Chiến lược tích hợp sẵn:** **6 Chiến lược chuẩn phương pháp QMV (`S1..S6`):** *Mua Hỗ Trợ KT01 (VPA)*, *Bắt Đáy An Toàn Đáy 2 Phân Kỳ*, *Ichimoku V7 (`6-18-36`)*, *Chu Kỳ Dòng Tiền Proxy*, *Tiết Cung & Tắc Nghẽn Bứt Phá*, *Hội Tụ Đa Cổng 3 Trụ Cột* (kèm 6 nút nạp bộ tham số tối ưu WFO Stage-4) + **6 Chiến lược Định lượng cơ sở** (`SMA/EMA`, `RSI Mean-Rev`, `MACD`, `BB Squeeze`, `Donchian ATR`, `RSI+BB`).
  - **Vi cấu trúc khớp lệnh thực tế TTCK Việt Nam (`engine.rs:1637-1997`):** Khớp tại `Open[i+1]` không nhìn trước tương lai, tính thuế bán `0.10%` + phí `0.15%`, làm tròn bước giá HOSE/HNX, kiểm tra khóa `T+2` (kể cả `T+3` lịch sử trước tháng 8/2022!), **tự động chặn lệnh bán thoát hàng nếu cổ phiếu bị sàn cứng mất thanh khoản (`trắng bên mua` — `UnfilledReason::LimitLocked`)**, và ưu tiên xử lý chạm `SL` trước khi nến chạm đồng thời cả `SL` lẫn `TP`.
  - **Hiển thị trực quan:** Biểu đồ SVG đường cong vốn (`Equity`) + sụt giảm (`Underwater Drawdown`) chia vạch `In-Sample / Out-of-Sample`, đánh dấu mũi tên Mua/Bán lên thẳng biểu đồ nến chính và bấm vào dòng lệnh để cuộn biểu đồ tới đúng ngày vào lệnh.
  - **Kiểm định Danh mục đa mã (`Portfolio Backtest`), Walk-Forward (`WFO`) & `Monte Carlo`:** Chạy danh mục trên 19 rổ QMV/Ngành với tỷ trọng đều `1/N` hoặc nghịch đảo biến động `1/σ` (có cảnh báo thiên kiến sống sót `Survivor Bias`); kiểm định trượt cửa sổ WFO tính hiệu suất Pardo `WFE %`; và mô phỏng `2.000` kịch bản Monte Carlo tính xác suất phá sản & `VaR95 / CVaR95`.
- `⚠️ [Các tính năng đã có trong `sumi-core::backtest` nhưng bị giới hạn / chưa mở lên UI]:`
  1. **Trình xuất Báo cáo HTML / In PDF chuẩn Tổ chức (`crates/sumi-core/src/backtest/report_html.rs`, `1.037` dòng):** Tạo báo cáo độc lập tuyệt đẹp có biểu đồ SVG, bảng phân rã lợi nhuận theo từng Năm, phân tích tác động ngoại lai (`OutlierImpact`) và nhật ký lệnh — **chưa có nút Xuất Báo Cáo trên UI**!
  2. **Quét lưới tham số tự động `run_parameter_sweep` (`12–64` biến thể) & Đấu trường so sánh toàn bộ 13 chiến lược `run_all_presets_battle` (`engine.rs:714-825`):** Có sẵn trong `sumi-core`, chưa nối lên `commands/backtest.rs` & UI.
  3. **Rule Builder tự thiết kế bị thu hẹp ở cầu nối Tauri (`commands/backtest.rs:250-314`):** Lõi `sumi-core` hỗ trợ cây biểu thức 2 tầng (`4` nhóm × `8` mệnh đề `AND/OR`) cho cả `Long` và `Short`, nhưng UI và cầu nối `backtest.rs` mới chỉ nối 1 nhóm phẳng `Long-only`, chỉ cho 1 điều kiện thoát lệnh không có ngưỡng số thực, và ẩn các tín hiệu ngoài nhóm `isQmvCore`.
  4. Tab `Walk-Forward` khóa cứng `5 folds, IS = 70%` và bỏ qua bảng chi tiết từng fold; tab `Monte Carlo` chưa vẽ biểu đồ quạt phân vị `checkpointEnvelopes` dù Rust đã trả dữ liệu về.

---

# CHƯƠNG 3 — MA TRẬN ĐỐI CHIẾU SUMI `v2.2.0` VS. TRADINGVIEW (`11 PHÂN HỆ CỐT LÕI`)

## 3.1. Bảng Đối Chiếu Khoảng Cách Tính Năng (`TradingView Parity & Gap Matrix`)

Thang đánh giá độ phức tạp kỹ thuật để phát triển thêm cho Sumi:
- 🟢 **`Dễ`** (`1–3 ngày`): Đã có sẵn hàm/dữ liệu trong `sumi-core` hoặc `tradingview-rs`, chủ yếu nối Tauri command + dựng UI React/KLineChart.
- 🟡 **`Trung bình`** (`4–10 ngày`): Cần viết thêm logic tổng hợp chuỗi thời gian, overlay KLineChart tùy chỉnh hoặc mở rộng SQLite schema.
- 🟠 **`Khó`** (`2–4 tuần`): Cần thiết kế trình thông dịch ngôn ngữ DSL, kiến trúc đa luồng hoặc đại tu luồng khớp lệnh.
- 🔴 **`Rất khó (Cần nguồn data mới)`**: Bắt buộc phải tích hợp nguồn dữ liệu nội địa mới (như DNSE Lightspeed MQTT / Yumeko / FireAnt) vì dữ liệu OHLCV hiện tại không chứa thông tin gốc.

### Nhóm 1 — Các loại Biểu đồ nâng cao & Khung thời gian tùy chỉnh

| Tính năng trên TradingView (Plus/Premium) | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Biểu đồ phi thời gian (`Renko, Line Break, Kagi, Point & Figure, Range Bars`)** | ❌ Chưa có trên UI | `tradingview-rs` có enum `ChartType::{Renko, LineBreak...}` nhưng phụ thuộc server TV; tốt hơn nên viết bộ biến đổi nến nhân quả trong `sumi-core` giống `heikinAshi.ts` để chạy được cả Offline & Replay. | 🟡 **Trung bình** |
| **`Baseline` / `HLC Area` / `Volume Candles` (độ rộng nến theo Vol)** | ❌ Chưa có | Rất hữu ích cho VSA/QMV (`Volume Candles` nhìn ngay nến nỗ lực lớn kết quả nhỏ). Có thể vẽ bằng custom series/overlay trên KLineChart. | 🟢 **Dễ – Trung bình** |
| **Time Price Opportunity (`TPO / Market Profile`)** | ⚠️ **Đã có lõi trong `sumi-core`, chưa lên UI** | `crates/sumi-core/src/indicators/market_profile.rs` đã tính xong `TPO`, `POC`, `VAH/VAL`, `IB`, `Naked POC`! Chỉ thiếu Tauri command + hàm `draw()` trên KLineChart. | 🟢 **Dễ** *(đã có 80% trong Rust)* |
| **Volume Footprint / Orderflow từng mức giá trong thân nến** | ❌ Chưa có Footprint thật *(chỉ có `ladder.rs` xấp xỉ)* | Footprint chuẩn đòi hỏi dữ liệu khớp lệnh từng tick (`Active Buy vs Active Sell` tại từng bước giá). Nếu làm từ `1m` OHLCV chỉ là xấp xỉ. | 🔴 **Rất khó (Cần nguồn tick data DNSE)** |
| **Khung thời gian tùy chỉnh (`2D, 3D, 2h, 45m, 3M`) & Giây (`1s..30s`)** | ❌ Mới cố định 9 khung (`1m..1M`) | Khung `2h, 45m, 2D, 3D` tổng hợp lên từ `15m / 1h / 1D` bằng `aggregate_ohlcv` rất gọn; khung giây (`s`) cần tài khoản TV Premium hoặc tick stream DNSE. | 🟡 **Trung bình** *(cho `2h/45m/2D/3D`)* |
| **Nút chuyển nhanh `ADJ / B-ADJ` (Giá điều chỉnh vs Giá gốc) ngay dưới biểu đồ** | ⚠️ **Có schema DB & file CSV Raw, chưa mở UI** | File `CafeF.SolieuGD.Raw.Upto06112026.csv` đã nằm sẵn trên đĩa! Chỉ cần bật `include_raw: true`, thêm nút `ADJ` ở góc dưới biểu đồ và truyền tham số `adjustment` xuống `get_bars`. | 🟢 **Dễ** |

### Nhóm 2 — So sánh Liên thị trường & Biểu đồ Công thức (`Compare & Spread/Ratio Charts`)

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Nút `Compare (+)` chồng nhiều mã (`VNINDEX`, mã cùng ngành, `DXY`) lên cùng 1 biểu đồ theo `%`** | ❌ Chưa có *(mới chỉ có chỉ báo phụ `RS_VNINDEX`)* | Cần tải chuỗi OHLCV của mã phụ, căn chỉnh theo `timestamp`, quy đổi sang `% thay đổi` tính từ mép trái khung nhìn và vẽ đường chồng trên `candle_pane`. | 🟡 **Trung bình** |
| **Biểu đồ Công thức Toán học (`HOSE:SSI / HOSE:VNINDEX`, `HNX:VN30F1! - HOSE:VN30`, `(HOSE:HPG + HOSE:HSG)/2`)** | ❌ Chưa có | Vũ khí cực mạnh để vẽ **Đường sức mạnh tương đối dạng nến (`Ratio Chart`)** và **Độ lệch Phái sinh (`VN30F Basis`)**! Cần bộ phân tích cú pháp biểu thức mã (`Expression Symbol Parser`) ở tầng `sumi-data` / `commands/market.rs` tổng hợp OHLC theo giao điểm `timestamp`. | 🟡 **Trung bình** |

### Nhóm 3 — Hệ sinh thái Chỉ báo, Ngôn ngữ Kịch bản & Hồ sơ Khối lượng

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Trình biên tập kịch bản tùy chỉnh (`Pine Script` / Custom Formula DSL ngay trong app)** | ❌ Chưa có | Hiện muốn thêm chỉ báo mới phải sửa code TypeScript/Rust và build lại app. Xây dựng 100% Pine Script v5 là bất khả thi/không cần thiết, nhưng làm một **QMV Formula / Mini-Pine DSL** (hỗ trợ `sma, ema, rsi, highest, lowest, crossover, plot, bgcolor` chạy trên WebWorker JS hoặc Rust AST) hoàn toàn khả thi. | 🟠 **Khó** |
| **Anchored VWAP (`AVWAP`) click chuột neo vào đỉnh/đáy/cây nến bùng nổ bất kỳ** | ⚠️ **Đã có thuật toán trong `sumi-core`, chưa nối lên công cụ vẽ** | `crates/sumi-core/src/indicators/vwap.rs:212` đã có sẵn `compute_anchored_vwap`! Chỉ cần thêm 1 custom overlay 1 điểm neo (`points: 1`) trên `LeftDrawingRail` để click chọn nến neo và vẽ đường `AVWAP ±1σ/2σ`. | 🟢 **Dễ** *(đã có lõi)* |
| **Fixed-Range Volume Profile (`FRVP`) kéo chọn 1 vùng tích lũy cụ thể** | ❌ Chưa có trên UI *(chỉ có `VRVP` toàn màn hình)* | Kết hợp overlay 2 điểm neo (`start_ts, end_ts`) với thuật toán chia giỏ giá + `POC / VAH / VAL 70%` sẵn có trong `volume_profile.rs` để đo chính xác giá vốn vùng tích lũy nền phẳng QMV. | 🟢 **Dễ – Trung bình** |
| **Session Volume Profile (`SVP` tự động theo từng ngày/tuần/tháng)** | ❌ Chưa có | Lặp qua từng ngày/tuần trên khung intraday (`15m/1h`) vẽ hồ sơ khối lượng từng phiên kèm đường nối `Developing POC`. | 🟡 **Trung bình** |
| **Indicator-on-Indicator (gắn đường `MA / Bollinger` đè lên `RSI / CCI / QMV_FLOW`)** | ❌ Chưa có *(ngoại trừ `OBV` & `RS` đã tích hợp sẵn 1 đường MA cố định)* | Cần cho phép chọn `Source = Output của chỉ báo khác` trong `IndicatorSettingsModal`. | 🟡 **Trung bình** |
| **Auto Chart Patterns / Auto Fib / Thống kê mùa vụ theo tháng (`Seasonality`)** | ⚠️ Mới có nhận diện `12` mẫu nến Nhật trong `patterns.rs` (chưa vẽ lên chart) | - **Seasonality (`Thống kê hiệu suất 12 tháng qua 10 năm CafeF`):** Rất dễ tính từ 10 năm lịch sử `1D` đang có sẵn trong `sumi_market.db`!<br>- **Auto Fib / Auto Wedge & Head-Shoulders:** Cần thuật toán ZigZag Pivot. | 🟢 **Dễ** *(Seasonality & Mẫu nến)* / 🟡 **Trung bình** *(Auto Patterns)* |

### Nhóm 4 — Công cụ Vẽ Chuyên sâu & Dựng Kịch bản Tương lai

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **`Ghost Feed` & `Bars Pattern` (vẽ nến giả định tương lai / sao chép mẫu sóng quá khứ)** | ❌ Chưa có *(nhưng đã có `AFC` dự phóng nón xác suất định lượng)* | `Bars Pattern` sao chép đoạn `N` cây nến quá khứ và kéo sang bên phải nến cuối cùng (tỷ lệ hóa theo giá) rất hợp để minh họa kịch bản sóng Elliott/Wyckoff. | 🟡 **Trung bình** |
| **`Path / Polyline` (vẽ đường kịch bản giá đi nhiều nhịp mũi tên), `Highlighter`, `Callout / Anchored Text`** | ⚠️ Có `anyWaves` & `brush`, nhưng thiếu hộp nhập chữ cho `Callout / Text` | Cần bổ sung hộp thoại soạn thảo văn bản trực tiếp cho `Callout / Price Note / Text` và công cụ `Path` có đầu mũi tên ở điểm cuối, cọ tô sáng bán trong suốt (`Highlighter`). | 🟢 **Dễ** |
| **`Andrews' Pitchfork / Schiff Pitchfork / Fib Time Zones / Gann Square`** | ⚠️ Đã có `Fib Retracement, Extension, Circle, Spiral, Fan, Gann Box` (7 công cụ); thiếu `Pitchfork` & `Fib Time Zones` | Viết thêm custom 3-point overlay cho `Pitchfork` và 2-point vertical lines cho `Fib Time Zones` (`1, 2, 3, 5, 8, 13, 21, 34...` phiên — rất hợp chu kỳ `T+` của QMV!). | 🟢 **Dễ** |
| **Thư mục gom nhóm hình vẽ (`Drawing Folders` trong Object Tree) & Lưu mẫu công cụ vẽ (`Drawing Templates`)** | ❌ Chưa có | Thêm `folder_id` / `name` vào `styles_json.__sumiMeta` trong bảng `drawings` và bảng `drawing_templates` để lưu bộ mức Fib yêu thích (`0.382, 0.5, 0.618` vs mức săn SL). | 🟢 **Dễ – Trung bình** |
| **Giữ `Shift + Click` đo nhanh khoảng giá / `%` / số nến rồi tự biến mất mà không mất công cụ đang chọn** | ❌ Chưa có *(hiện phải bấm `Shift+M` chuyển hẳn sang công cụ thước đo)* | Bắt sự kiện `e.shiftKey` + click trên tầng bọc `KLineViewport.tsx` vẽ hộp đo tạm thời (ephemeral measure overlay), click lần nữa tự xóa. Cực kỳ tiện tay! | 🟢 **Dễ** (`< 1 ngày`) |

### Nhóm 5 — Sự kiện Doanh nghiệp & Dữ liệu Cơ bản trên Trục Biểu đồ (`Events & Financials on Chart`)

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Mốc `D` (Cổ tức / GDKHQ), `S` (Chia tách / Thưởng CP), `E` (Ra BCTC Quý & EPS Surprise) dưới chân nến** | ❌ Chưa có | Với `D` và `S`: thậm chí có thể **tự động suy ra ngày GDKHQ lịch sử** bằng cách so tỷ số `Raw CSV / Adjusted CSV` đang có sẵn trên máy + lấy sự kiện sắp tới từ `tradingview-rs` / CafeF! Với `E`: lấy từ module `fundamental` (`FQ`) + `earnings_release_date` trong `tradingview-rs`. | 🟡 **Trung bình** |
| **Vẽ chỉ số tài chính (`P/E, P/B, ROE, Tăng trưởng LNST Quý`) thành đường chỉ báo dưới biểu đồ giá** | ❌ Chưa có | Dùng `tradingview-rs::fundamental::get_fundamental_data` tải chuỗi lịch sử Quý (`FQ`) / `TTM`, lưu bảng `fundamentals` trong `sumi_market.db`, sau đó chiếu dạng bậc thang (`step-line`) theo mốc thời gian dưới biểu đồ `1D / 1W` (tính `P/E động hằng ngày = Close[t] / EPS_TTM[t]`). | 🟡 **Trung bình** *(nhờ `tradingview-rs` đã có sẵn API!)* |

### Nhóm 6 — Watchlist Chuyên nghiệp, Bản đồ Nhiệt & Độ rộng Thị trường (`Watchlist Pro, Heatmap & Breadth`)

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Gắn Cờ màu phân loại mã (`Red / Orange / Green / Cyan / Purple Flags`) & Danh sách theo Cờ** | ❌ Chưa có | Thêm bảng `symbol_flags (ticker PRIMARY KEY, color)` trong `sumi_workspace.db`, click biểu tượng lá cờ đầu mỗi dòng Watchlist/Screener để đánh dấu *"Chờ mua" / "Đang nắm giữ" / "Dẫn dắt"*. | 🟢 **Dễ** |
| **Thanh tiêu đề chia nhóm con (`Section Headers` như `--- NGÂN HÀNG ---`), Kéo thả sắp xếp (`Drag & Drop`) & Sắp xếp theo cột** | ❌ Chưa có *(cột `sort_order` đã có sẵn trong SQLite `watchlist_items`!)* | Bảng `watchlist_items` (`workspace_db.rs:321`) **đã có sẵn cột `sort_order`**! Chỉ cần nối UI kéo thả + hỗ trợ phần tử phân cách `###SECTION:Tên Nhóm` + click tiêu đề cột sắp xếp theo `% Đổi / KL / RVOL`. | 🟢 **Dễ** |
| **Bản đồ nhiệt toàn thị trường (`Stock Heatmap` theo Vốn hóa × Ngành × `% Đổi` / `QMV Flow`)** | ❌ Chưa có | Sumi đã có sẵn phân nhóm `11 Ngành QMV` (`qmv_universe.rs`)! Kết hợp thêm `market_cap_basic` từ `tradingview-rs` `SymbolInfo` để vẽ Treemap tương tác (màu chuẩn VN gồm cả Tím Trần & Xanh lơ Sàn, hoặc tô màu theo dòng tiền `QMV_FLOW`!). | 🟡 **Trung bình** |
| **Chỉ báo Độ rộng thị trường nội tại (`% mã > MA20/MA50/MA200`, Số mã Tăng/Giảm, Biểu đồ xoay ngành `RRG`)** | ⚠️ **Đã lập trình xong trong `sumi-core/src/scanner/breadth.rs`, chưa nối lên UI!** | Mở Tauri command gọi `compute_market_breadth_and_rrg` là có ngay bảng điều khiển Độ rộng thị trường & Biểu đồ 4 góc phần tư `RRG` xoay vòng các ngành QMV quanh `VNINDEX`! | 🟢 **Dễ** *(đã có 85% trong Rust)* |

### Nhóm 7 — Bộ lọc Cổ phiếu (`Stock Screener 2.0`)

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Lọc Đa Khung Thời Gian đồng thời (`1W` thuận xu hướng `AND` `1D` cạn cung hỗ trợ `AND` `1H` cắt lên)** | ❌ Hiện khóa cứng duy nhất khung `1D` | Với tổ hợp **`1W + 1D`**: **làm được ngay lập tức với tốc độ `< 150ms`** vì `sumi-core` tổng hợp `1W` trực tiếp từ `1D` sẵn có trong SQLite! Riêng điều kiện kèm khung intraday (`1H`) cần tải bổ sung nến `1H` cho tập mã đã lọt qua phễu `1W + 1D` (kiến trúc lọc phễu 2 tầng `Two-Stage Funnel` để không bị nghẽn mạng cho 475 mã). | 🟡 **Trung bình** *(Rất thông minh nếu dùng phễu `1W+1D -> tải 1H cho Top 40 mã`!)* |
| **Lưu & Quản lý nhiều Bộ lọc cá nhân (`Saved Custom Screens`)** | ❌ Chưa nối SQLite *(trong `coaching.rs` có `SavedScanSnapshot` cũ chưa dùng)* | Thêm bảng `saved_screens` vào `sumi_workspace.db` lưu JSON cấu hình bộ lọc. | 🟢 **Dễ** |
| **Chế độ Tự động quét cập nhật (`Auto-refresh`) & Khắc phục dữ liệu EOD cũ** | ❌ Chưa có (đang bị kẹt nến cũ `06/11` cho 91% mã) | Cần thêm nút **"Đồng bộ EOD toàn rổ (`Sync Universe EOD`)"** tải bù các nến thiếu kể từ ngày cuối trong SQLite, kèm bộ hẹn giờ quét lại mỗi `1–5 phút` cho rổ `QMV60 / Watchlist`. | 🟡 **Trung bình** |
| **Xuất danh sách kết quả lọc thẳng thành 1 Watchlist mới chỉ bằng `1 click`** | ❌ Mới chỉ có nút xuất file `.csv` | Thêm nút `"Lưu thành Watchlist"` gọi `create_watchlist` + nạp mảng `rows.map(r => r.ticker)`. | 🟢 **Dễ** (`< 2 giờ`) |

### Nhóm 8 — Kiểm định Chiến lược (`Strategy Tester`) — Đối chiếu Công bằng Hai Chiều

#### A. Những tính năng TradingView có mà Sumi CHƯA có:
| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Giải pháp & Độ khó kỹ thuật |
|---|---|---|
| **Giải ngân từng phần / Mua gom 3 nhịp (`Pyramiding / Scale-in 30%–50%–20%`) & Chốt lời từng phần (`Scale-out 1/2 tại TP1, gồng 1/2 theo Trailing MA20/Kijun`)** | ❌ Chưa có trong `backtest/engine.rs` *(mặc dù `Paper Broker` `portfolio.rs:1422-1610` đã hỗ trợ mua thêm & chốt từng phần!)* | Hiện `backtest/engine.rs` mới giữ 1 vị thế đơn (`all-in / all-out`). Nâng cấp `ActivePositionState` để hỗ trợ mảng lô `T+2` mua bồi (`max_pyramiding = 3`) và chốt `50%` tại `TP1` rất sát triết lý đi vốn QMV! — 🟡 **Trung bình** |
| **`Bar Magnifier` (dùng nến khung nhỏ `1m/15m` giải quyết xung đột chạm `SL` hay `TP` trước trong 1 cây nến `1D`)** | ❌ Chưa có *(hiện dùng quy tắc bi quan an toàn: giả định chạm `SL` trước tại `engine.rs:1927-1955`)* | Khi một cây nến `1D` có biên độ trùm qua cả `SL` và `TP`, tự động truy vấn nến `15m`/`1h` của đúng ngày đó từ `sumi-data` để dò thứ tự chạm thật. (Lưu ý giới hạn độ sâu lịch sử `1m` của TV cho các năm xa). — 🟡 **Trung bình** |
| **Bảng nhiệt Lợi nhuận theo Tháng × Năm (`Monthly Performance Heatmap Table`)** | ❌ Chưa có *(trong `analytics.rs` mới có bảng theo Năm UTC)* | Tính lợi nhuận gộp từ `equity_curve` theo lưới `Năm × 12 Tháng` để nhìn rõ chiến lược thường thắng/thua vào mùa nào (ví dụ tháng Tết, mùa BCTC quý 2/quý 3). — 🟢 **Dễ** |
| **Xuất báo cáo chi tiết (`HTML / PDF / CSV Trade Log`)** | ⚠️ **Đã có sẵn `report_html.rs` (`1.037` dòng) trong `sumi-core`, chưa mở nút trên UI!** | Chỉ cần thêm 1 Tauri command bọc `render_backtest_html_report` + nút `"Xuất Báo Cáo HTML/PDF"` trên `StrategyTesterTab.tsx`! — 🟢 **Dễ** (`< 0.5 ngày`) |

#### B. Những vũ khí Sumi `v2.2.0` ĐÃ VƯỢT XA TradingView bản chuẩn (Đặc sản QMV & TTCK Việt Nam):
1. **Mô phỏng chuẩn xác Luật Thanh toán VSD `T+2` (`13:00 ICT`) & Lịch sử `T+3` trước 08/2022:** Pine Script trên TradingView mặc định cho phép mua hôm nay và cắt lỗ ngay sáng mai (`T+1`), dẫn đến kết quả Backtest cổ phiếu Việt Nam trên TradingView bị **ảo tưởng lợi nhuận và đánh giá thấp rủi ro kẹp hàng `T+`**. Sumi khóa chặt quyền bán tự nguyện cho tới đúng phiên hàng về theo lịch nghỉ lễ Việt Nam!
2. **Mô phỏng Kẹt Thanh Khoản Sàn Cứng (`Trắng bên mua` — `LimitLocked`):** Khi cổ phiếu mở cửa và đóng cửa cứng ở giá Sàn không có cầu đối ứng (`engine.rs:1638-1665`), TradingView vẫn khớp lệnh thoát ảo ngay giá mở cửa, còn Sumi **chặn lệnh bán và đẩy sang phiên kế tiếp** đúng như nỗi đau thật trên sàn HOSE!
3. **Bước giá bậc thang HOSE (`10 / 50 / 100 VND`), Lô chẵn `100` cp, Thuế bán `0.10%` & Ký quỹ `17%` `VN30F1!`:** Pine Script không hỗ trợ bước giá biến thiên theo mức thị giá của HOSE.
4. **Hệ thống Kiểm định Định lượng Chống Học Vẹt (`Anti-Overfitting Lab`) tích hợp gốc:**
   - Tự động cắt `In-Sample / Out-of-Sample` trên mọi lần chạy.
   - **Walk-Forward Optimization (`WFO` 5-Fold)** chấm điểm `Walk-Forward Efficiency (WFE %)` và độ ổn định tham số.
   - **Monte Carlo `2.000` kịch bản** tính xác suất cháy tài khoản và `CVaR95` của sụt giảm vốn.
   - **Portfolio Backtest đa mã đồng thời** (`QMV30`, `QMV60`, từng nhóm ngành với phân bổ `1/N` hoặc `Inverse Volatility 1/σ`) — trong khi Strategy Tester của TradingView chỉ chạy được **từng mã đơn lẻ một**!

### Nhóm 9 — Bar Replay

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Nhảy tới ngày cụ thể bằng Lịch (`Jump to Specific Date/Time`)** | ❌ Mới chỉ chọn bằng kéo thanh rèm trên biểu đồ hoặc thanh trượt | Thêm `<input type="date">` trên `ReplayControlBar.tsx` truyền `timestampMs` vào hàm `jumpReplay` (`replay_seek` đã viết sẵn trong Rust!). | 🟢 **Dễ** (`< 2 giờ`) |
| **Chạy Replay Ngẫu nhiên Mã + Ngẫu nhiên Thời điểm + Ẩn danh hoàn toàn (`True Blind Practice`)** | ⚠️ `Blind Mode` mới chống lộ giá tương lai, chưa ẩn tên mã/ngày/mức giá và chưa bốc ngẫu nhiên | Thêm nút **"🎲 Luyện Phản Xạ Mù (`Random Blind Drill`)"**: chọn ngẫu nhiên 1 mã trong rổ `QMV60` + ngẫu nhiên 1 ngày trong `2018–2025`, đổi nhãn mã thành `MÃ ẨN #A7`, dịch ngày thành `Phiên #1..#N`, nhân giá với hằng số chuẩn hóa `100.0` tại điểm cắt, chỉ tiết lộ mã thật khi kết thúc phiên! | 🟢 **Dễ – Trung bình** |
| **Bảng Tổng kết Điểm số cuối phiên luyện tập (`Replay Session Debrief`)** | ⚠️ **Đã viết sẵn trong `sumi-core/src/trading/coaching.rs`, chưa nối lên UI!** | Nối `SessionDebriefReport` lên modal tổng kết khi bấm `"Thoát Replay"`: chấm điểm tuân thủ `SL`, tỷ lệ `Win Rate`, `Avg R`, so sánh hiệu suất người tập với `Buy & Hold` trong cùng đoạn nến! | 🟢 **Dễ** *(đã có lõi)* |
| **Đồng bộ Đa khung giờ nhân quả & Lưu phiên Replay kèm Sổ lệnh** | ✅ **Sumi ĐÃ CÓ & VƯỢT TradingView Essential/Plus** | Sumi cho phép lưu nhiều phiên Replay đang tập dở (kèm cả vị thế Paper Broker) vào SQLite để hôm sau mở lại tập tiếp — tính năng mà ngay cả TradingView cũng không lưu kèm trạng thái sổ lệnh luyện tập theo từng phiên. | ✅ **Đã hoàn thành** |

### Nhóm 10 — Hệ thống Cảnh báo (`Alerts`)

| Tính năng trên TradingView | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Đặt cảnh báo trên mọi tín hiệu / chỉ báo (`123` tín hiệu QMV, Ichimoku, Phân kỳ, VSA, `QMV_FLOW`)** | ⚠️ **Đã có trong `sumi-core::alerts::AlertEngine`, chưa nối lên Tauri/UI** (UI mới chỉ có `RSI, MACD, EMA`) | Thay thế bộ kiểm tra thủ công trong `commands/workspace.rs` bằng `sumi_core::alerts::AlertEngine` (`SignalCatalogTrigger` + `ConfluenceThreshold`). | 🟢 **Dễ – Trung bình** |
| **Cảnh báo trên cả một Watchlist (`Watchlist Alerts` — 1 cảnh báo canh tự động toàn bộ `QMV30 / QMV60`)** | ❌ Chưa có | Tác vụ chạy ngầm định kỳ trong Rust quét danh sách mã của Watchlist bằng `AlertEngine`, bắn thông báo khi bất kỳ mã nào trong rổ `QMV30` nổ tín hiệu! | 🟡 **Trung bình** |
| **Thông báo đẩy ra ngoài khi thu nhỏ app (`macOS Native Notification / Telegram Bot / Webhook`)** | ⚠️ Mới có chuông trong Webview (chưa xin quyền OS Notification, chưa có Telegram/Webhook) | Tích hợp `tauri-plugin-notification` cho macOS Banner + cho phép người dùng nhập `Telegram Bot Token & Chat ID` / `Webhook URL` trong Cài đặt để bắn tin nhắn điện thoại qua `reqwest` sẵn có trong Rust! | 🟢 **Dễ** |
| **Điều kiện đa hợp lưu (`Combined Conditions`) & Chế độ lặp `Once Per Bar Close` + Nhật ký Alert Log** | ⚠️ Đã có `OncePerBarClose` + `cooldown_bars` + `AlertTriggerEvent` trong `sumi-core` | Mở giao diện cấu hình tần suất lặp, sửa cảnh báo và tab Lịch sử nổ cảnh báo (`Alert Log`). | 🟢 **Dễ** |

### Nhóm 11 — Trải nghiệm Không gian Làm việc & Đa Màn Hình (`Workspace UX`)

| Tính năng trên TradingView Desktop | Trạng thái trên Sumi `v2.2.0` | Khoảng cách & Ghi chú kiến trúc | Độ khó kỹ thuật |
|---|---|---|---|
| **Phóng to tạm thời 1 ô biểu đồ trong lưới 4 ô lên toàn màn hình (`Maximize Single Pane` bằng `Alt+Enter` / nút góc ô rồi thu về lưới 4 ô)** | ❌ Chưa có *(hiện chỉ có phóng to khung Dock bên dưới)* | Thêm state `maximizedSlotId: string \| null` vào `workspaceStore.ts`: khi kích hoạt thì ẩn tạm 3 ô kia bằng CSS mà không unmount hay mất trạng thái lưới `4` ô. | 🟢 **Dễ** (`< 2 giờ`) |
| **`Cmd+K` Command Palette thực thi lệnh nhanh toàn hệ thống (đổi khung, bật chỉ báo, áp dụng mẫu QMV, mở Replay, đổi giao diện...)** | ⚠️ `Cmd+K` hiện chỉ mở hộp thoại tìm mã (`SymbolSearchModal`) | Nâng cấp `Cmd+K` thành bảng điều khiển đa năng (gõ `> ichimoku`, `> replay`, `> dark`, `> screener`, hoặc gõ tên mã `FPT`). | 🟢 **Dễ** |
| **Đồng bộ lăn chuột & độ thu phóng trục thời gian chính xác giữa các khung giờ khác nhau (`1D <-> 1h`)** | ⚠️ Mới đồng bộ tâm thời gian (`scrollToTimestamp`), chưa đồng bộ khoảng thời gian hiển thị `[fromTs, toTs]` | Chống lệch tỷ lệ khi cuộn giữa ô `1D` và ô `15m`, kèm sửa 2 lỗi cửa sổ phụ (`capabilities/default.json` & khóa `stream_pane-0`). | 🟢 **Dễ – Trung bình** |
| **Lịch sử `Undo / Redo` toàn diện (bao gồm cả xóa nhầm chỉ báo, đổi bố cục)** | ⚠️ Mới chỉ Undo/Redo cho Hình vẽ trên từng mã | Mở rộng stack hoàn tác ở cấp `workspaceStore` cho thao tác xóa chỉ báo / áp dụng nhầm mẫu chỉ báo. | 🟢 **Dễ – Trung bình** |

---

# CHƯƠNG 4 — ĐỀ XUẤT TOP 14 TÍNH NĂNG ĐÁNG PHÁT TRIỂN NHẤT CHO SUMI (`HIGH-ROI QMV ROADMAP`)

Loại bỏ hoàn toàn các tính năng mạng xã hội rườm rà không cần thiết của TradingView (*Ideas cộng đồng, Phòng chat, Livestream, Broker quốc tế*), danh sách 14 hạng mục dưới đây được xếp hạng nghiêm ngặt theo công thức **`Giá trị thực chiến cao nhất cho phương pháp QMV trên TTCK Việt Nam × Độ khả thi kỹ thuật cao nhất`** (ưu tiên mở khóa trước các tài sản ngầm đã hoàn thành 70–90% trong `sumi-core` và `tradingview-rs`):

## Tầng 1 — "Quick Wins" Hiệu Suất Đột Biến (Khai quật kho báu có sẵn & Vá điểm nghẽn dữ liệu — `1 đến 3 ngày / mục`)

| Hạng | Tên tính năng đề xuất | Tại sao cực kỳ đáng làm ngay? | Tận dụng tài sản có sẵn nào? | Độ khó |
|---:|---|---|---|---|
| **#1** | **Khắc phục "Bẫy Dữ Liệu EOD Cũ" + Đồng bộ đủ `~1.600` mã VN & Sửa 4 lỗi nền tảng** | Giải quyết ngay việc `91,6%` mã trong Screener đang kẹt ở ngày `06/11`; tự động cập nhật EOD toàn rổ `QMV60 / ALL_VN` trước khi quét; đồng thời vá ngay 4 lỗi nhỏ nhưng chí mạng: (1) dịch ngược `HNX:VN301! -> HNX:VN30F1!` cho quote phái sinh, (2) cấp quyền `capabilities/default.json` + tách khóa stream cho cửa sổ phụ, (3) thêm `'30m'` vào `INTRADAY_INTERVALS`, (4) sửa câu SQL `cafef_spliced` khi chia tách lần 2. | `tradingview-rs::list_symbols`, `cache_service.rs`, `scanner.rs` | 🟢 **Dễ** |
| **#2** | **Mở khóa `Anchored VWAP (AVWAP)` + `Fixed-Range Volume Profile (FRVP)` + `TPO Market Profile` lên Biểu đồ** | Ba vũ khí đọc giá vốn tổ chức và vùng kẹp hàng đỉnh cao nhất cho dân PTKT & VSA/QMV: click neo `AVWAP` vào cây nến bùng nổ/đáy rũ bỏ, kéo `FRVP` qua nền tích lũy 3 tháng để tìm đúng `POC + VAH/VAL 70%`. | **Đã viết sẵn 80% toán học** trong `sumi-core/src/indicators/{vwap.rs, volume_profile.rs, market_profile.rs}`! | 🟢 **Dễ** |
| **#3** | **Mở bảng điều khiển Độ Rộng Thị Trường (`Market Breadth`) & Biểu đồ Xoay Ngành 4 Góc Phần Tư (`RRG`)** | Trả lời tức thì câu hỏi sống còn mỗi sáng của phương pháp QMV: *"Thị trường đang lan tỏa thật hay kéo trụ xanh vỏ đỏ lòng? Ngành nào (Chứng khoán, Ngân hàng, Thép, BĐS) đang chuyển từ `Improving` sang `Leading` so với `VNINDEX`?"* | **Đã viết xong 100% lõi tính toán** trong `crates/sumi-core/src/scanner/breadth.rs` + phân ngành `qmv_universe.rs`! | 🟢 **Dễ** |
| **#4** | **Nút chuyển nhanh `ADJ` (Giá điều chỉnh vs Giá gốc) + Tự động gắn mốc `D / S` (Cổ tức / Chia tách) dưới chân nến** | Trên TTCK Việt Nam, các mức kháng cự tâm lý mệnh giá (`10.000, 20.000, 50.000, 100.000 VND`) phụ thuộc rất mạnh vào **Giá thực trên bảng điện (`Unadjusted`)**, trong khi đường MA dài hạn cần **Giá điều chỉnh (`Adjusted`)**. Chuyển `1-click` giữa hai chế độ là nhu cầu thực chiến hằng ngày! | File `CafeF.SolieuGD.Raw.Upto06112026.csv` (`940k` dòng) & schema cột `adjustment` đã nằm sẵn trên máy! | 🟢 **Dễ** |
| **#5** | **Nối `AlertEngine` (`123` tín hiệu QMV) + Cảnh báo trên cả Rổ `Watchlist` + Bắn tin nhắn `Telegram` / macOS Banner** | Biến Sumi thành trợ lý canh bảng tự động: đặt 1 cảnh báo duy nhất *"Báo về Telegram ngay khi bất kỳ mã nào trong rổ `QMV30` xuất hiện Hỗ trợ cạn cung hoặc Vượt mây Ichimoku V7 `6-18-36` trên khung `1H/1D`"*. | **Đã có sẵn `AlertEngine` (`SignalCatalogTrigger`)** trong `sumi-core/src/alerts/engine.rs` + `reqwest` trong Rust! | 🟢 **Dễ – Trung bình** |
| **#6** | **Mở nút Xuất Báo Cáo Backtest `HTML / PDF` + Bảng Nhiệt Lợi Nhuận Tháng/Năm + Mở lệnh `STOP` trên Paper Broker** | Tận dụng ngay hơn `1.000` dòng code báo cáo chuẩn tổ chức đang ngủ yên trong `report_html.rs`, thêm lệnh chờ `STOP` đã có sẵn ở backend lên phiếu đặt lệnh UI, và vẽ thêm quạt phân vị Monte Carlo. | **Đã có sẵn 95%** trong `backtest/report_html.rs` và `commands/broker.rs`! | 🟢 **Dễ** |

---

## Tầng 2 — Vũ Khí Phân Tích Chuyên Sâu, Screener Đa Khung & Quản Trị Vốn QMV (`4 đến 10 ngày / mục`)

| Hạng | Tên tính năng đề xuất | Giá trị thực chiến cho phương pháp QMV & TTCK Việt Nam | Giải pháp kiến trúc | Độ khó |
|---:|---|---|---|---|
| **#7** | **Bộ lọc Cổ phiếu Đa Khung Thời Gian (`MTF Screener 2.0`: `1W + 1D + 1H`) + Lưu Bộ lọc & Xuất thẳng ra Watchlist** | Tinh hoa của QMV nằm ở sự đồng pha đa chu kỳ: *Khung Tuần (`1W`) nằm trên mây Kumo `AND` Khung Ngày (`1D`) vừa điều chỉnh tiết cung về MA20 `AND` Khung Giờ (`1H`) nảy lên cắt đường tín hiệu*. | Thiết kế **Phễu 2 tầng (`Two-Stage Funnel`)**: Tầng 1 lọc tức thì `1W + 1D` từ SQLite (`< 150ms`), Tầng 2 tự động tải/kiểm tra `1H` cho `20–50` mã vượt qua vòng 1; thêm lưu Preset lọc & 1-click tạo Watchlist. | 🟡 **Trung bình** |
| **#8** | **Khai thác Module `fundamental` của `tradingview-rs`: Chỉ số Tài chính (`P/E, P/B, ROE, Tăng trưởng LNST`) trên Screener, Thẻ mã & Biểu đồ** | Giúp loại bỏ cổ phiếu "rác" bo cung, lọc doanh nghiệp tăng trưởng lợi nhuận quý đột biến kết hợp điểm mua kỹ thuật QMV, đồng thời vẽ dải định giá lịch sử `P/E` & `P/B` ngay dưới biểu đồ giá! | Gọi `tradingview-rs::fundamental::fetch_full_fundamentals` + giữ lại các trường `SymbolInfo` (`market_cap_basic, price_earnings_ttm, eps`) lưu vào bảng `fundamentals` trong `sumi_market.db`. Không tốn 1 đồng phí API! | 🟡 **Trung bình** |
| **#9** | **Nâng cấp `Strategy Tester`: Mua gom từng nhịp (`Pyramiding / Scale-in`) + Chốt lời từng phần (`Scale-out`) + `Bar Magnifier`** | Thực chiến trên sàn VN theo QMV không bao giờ "mua 1 cục — bán 1 cục", mà **mua thăm dò `30%` ở nền hỗ trợ, gia tăng `70%` khi nổ Vol vượt cản (mỗi lô tính `T+2` riêng!), chốt `1/2` ở cản đỉnh cũ và gồng `1/2` theo `MA20 / Kijun(18)`**. | Nâng cấp `ActivePositionState` trong `backtest/engine.rs` theo mô hình danh sách lô FIFO giống `trading/portfolio.rs`, kèm tùy chọn dò nến `15m/1h` (`Bar Magnifier`) cho các nến chạm kép `SL & TP`. | 🟡 **Trung bình** |
| **#10** | **Biểu đồ Công thức & So sánh Liên thị trường (`Compare +` & `HOSE:SSI / HOSE:VNINDEX`, `HNX:VN30F1! - HOSE:VN30`)** | Cho phép vẽ trực tiếp **Biểu đồ nến Sức mạnh tương đối** của cổ phiếu chia cho `VNINDEX` (kẻ được cả Trendline và Ichimoku trên đường RS!), so sánh sức mạnh các cổ phiếu trong cùng 1 ngành, và theo dõi trực tiếp **độ lệch `Basis` phái sinh**. | Bộ tính chuỗi tổng hợp (`Synthetic Spread/Ratio Series Engine`) ở `sumi-data` + lớp phủ `Compare (%)` trên `KLineViewport`. | 🟡 **Trung bình** |
| **#11** | **`True Blind Replay Drill` (Bốc ngẫu nhiên Mã + Thời điểm + Ẩn danh 100%) + Nối Bảng Chấm Điểm `Session Debrief` & `Trade Planner`** | Nâng cấp `Bar Replay` thành phòng tập phản xạ chuyên nghiệp không thiên kiến nhận thức: che hoàn toàn tên mã/năm lịch sử, buộc người tập tính khối lượng đi vốn chuẩn rủi ro `R` qua `planner.rs`, và xuất phiếu chấm điểm kỷ luật (`coaching.rs`) cuối buổi tập. | Kết hợp ẩn danh chuỗi giá ở `ReplayControlBar` với hai module **đã viết sẵn 100% trong `sumi-core`**: `journal/planner.rs` và `trading/coaching.rs`! | 🟢 **Dễ – Trung bình** |
| **#12** | **Nâng cấp UX Tốc độ cao: `Shift + Click` đo nhanh, `Alt + Enter` phóng to 1 ô biểu đồ, Watchlist Cờ màu / Chia nhóm / Kéo thả & Bản đồ nhiệt `QMV Sector Heatmap`** | Hoàn thiện cảm giác sử dụng mượt mà ngang ngửa TradingView Desktop hằng ngày: giữ `Shift` đo biên độ sóng trong 1 giây không mất công cụ vẽ, gắn cờ Đỏ/Cam/Xanh phân loại danh mục săn cổ phiếu, và nhìn toàn cảnh dòng tiền bằng Treemap 11 ngành QMV. | Nâng cấp thuần Frontend React/Zustand + bảng `symbol_flags` trong `sumi_workspace.db`. | 🟢 **Dễ – Trung bình** |

---

## Tầng 3 — Đột Phá Chiến Lược Dài Hạn (`2 đến 4 tuần`)

| Hạng | Tên tính năng đề xuất | Ý nghĩa chiến lược dài hạn | Giải pháp kiến trúc | Độ khó |
|---:|---|---|---|---|
| **#13** | **Cầu nối Dữ liệu Thời gian thực Nội địa (`Yumeko / DNSE Lightspeed Bridge` qua `yumeko-sumi-contract`)** | Xóa bỏ hoàn toàn độ trễ `15 phút` của TradingView trên sàn Việt Nam, mở khóa **Khớp lệnh từng tick (`Time & Sales`)**, **Mua/Bán chủ động thật** (thay thế `OHLCV Proxy`), **Sổ lệnh 3/10 bước giá**, và **Dòng tiền Khối ngoại / Tự doanh**. | Nối `sumi-data` với hạ tầng `yumeko-source-dnse` / `yumeko-sumi-contract` đã có sẵn trong hệ sinh thái `/Users/mizuhara/workspace/yumeko` theo kiến trúc `Multi-Provider` (`provider.rs`). | 🟠 **Khó** *(nhưng đã có nền tảng bên Yumeko!)* |
| **#14** | **Trình biên tập Công thức & Tín hiệu Tùy biến (`Sumi QMV Script / Formula Editor`)** | Trao quyền cho người dùng tự viết chỉ báo riêng, tự định nghĩa điều kiện lọc cổ phiếu và chiến lược kiểm định bằng cú pháp ngắn gọn kiểu `Pine-lite` ngay trong ứng dụng mà không cần biên dịch lại Rust/TypeScript. | Mở rộng bộ biên dịch biểu thức sẵn có trong `sumi-core::backtest::strategy` thành một ngôn ngữ công thức mảng (`Vectorized Series DSL`) kèm trình soạn thảo có gợi ý cú pháp ngay trong Dock dưới. | 🟠 **Khó** |

---

*Báo cáo được trích xuất và kiểm chứng trực tiếp 100% từ mã nguồn hiện hành của `Sumi Supercharts v2.2.0` và cơ sở dữ liệu thực tế `sumi_market.db` / `sumi_workspace.db` ở chế độ chỉ đọc tuyệt đối.*
