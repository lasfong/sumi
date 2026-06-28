# TÀI LIỆU YÊU CẦU SẢN PHẨM (PRD) & THIẾT KẾ HỆ THỐNG

> Trạng thái tài liệu: **Long-term vision / Future scope**.
>
> Tài liệu này mô tả tầm nhìn đầy đủ và nhiều năng lực dài hạn của dự án. Nó không còn là hợp đồng triển khai trực tiếp cho MVP hiện tại. Với công việc hiện tại, dev phải ưu tiên đọc `docs/INDEX.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` và `docs/IMPLEMENTATION_PLAYBOOK.md`.
>
> Nếu tài liệu này mâu thuẫn với `docs/PRODUCT_SPEC.md` hoặc `docs/DECISIONS.md`, hãy theo `PRODUCT_SPEC.md` và `DECISIONS.md`.

## DỰ ÁN: VIETNAM STOCK BACKTESTING & TRADING REPLAY SYSTEM (VN-FIN-BACKTEST)

---

## 🛠️ PHẦN 1: TỔNG QUAN HỆ THỐNG & PHẠM VI (SCOPE)

### 1.1 Mục tiêu sản phẩm

Hệ thống **VN-FIN-BACKTEST** là một nền tảng Web Application chuyên nghiệp phục vụ cho hai mục đích cốt lõi:

1. **Manual Bar Replay (Luyện tập thủ công):** Cho phép nhà đầu tư tua lại lịch sử thị trường, che giấu tương lai, thực hiện phân tích kỹ thuật bằng tay và đặt lệnh giả lập từng phiên để rèn luyện tâm lý và kỹ năng giao dịch.
2. **Systematic Algorithmic Backtesting (Kiểm thử thuật toán):** Cho phép lập trình viên/AI Engineer viết chiến lược bằng mã nguồn Python, chạy mô phỏng trên dữ liệu lịch sử quy mô lớn để đánh giá hiệu suất qua các chỉ số tài chính tiêu chuẩn.

### 1.2 Đối tượng người dùng (User Personas)

* **Discretionary Traders:** Nhà giao dịch theo trường phái phân tích kỹ thuật, cần môi trường "Replay" sạch để backtest hệ thống giao dịch bằng tay mà không bị thiên kiến tương lai (Look-ahead bias).
* **Quant / AI Engineers / System Traders:** Lập trình viên muốn tự động hóa chiến lược, tối ưu hóa bộ tham số (Parameters Tuning) dựa trên dữ liệu lịch sử trước khi đưa vào hệ thống Live Trading.

### 1.3 Phạm vi và Ràng buộc thị trường Việt Nam (Market Constraints)

Hệ thống được thiết kế may đo riêng cho thị trường chứng khoán Việt Nam (HOSE, HNX, UPCoM), bắt buộc phải xử lý cấu trúc nghiệp vụ đặc thù:

* **Chu kỳ thanh toán (T+1.5 / T+2):** Cổ phiếu mua ngày $T$ chỉ được phép bán vào phiên chiều ngày $T+2$ (sau 13:00). Động cơ backtest phải khóa trạng thái bán đối với khối lượng cổ phiếu đang trên đường về.
* **Biên độ trần/sàn (Price Limits):** Khóa trần/sàn theo từng sàn: HOSE ($\pm7\%$), HNX ($\pm10\%$), UPCoM ($\pm15\%$). Lệnh Limit Order đặt ngoài biên độ này sẽ bị hệ thống từ chối (Reject).
* **Cơ chế khớp lệnh ATC/ATO:** Giá mở cửa (Open) và đóng cửa (Close) được quyết định bởi các phiên khớp lệnh định kỳ.
* **Thuế và Phí (Friction Cost):** Cấu hình mặc định: Phí mua ($0.15\%$), Phí bán ($0.15\%$), Thuế thu nhập khi bán ($0.1\%$). Tổng chiều bán chịu chi phí $0.25\%$.
* **Không bán khống (No Short Selling):** Đối với thị trường cơ sở (Stock), vị thế bán khống bị cấm. Hệ thống chỉ hỗ trợ lệnh `BUY` (Mở vị thế Long) và `SELL` (Đóng vị thế Long). Chế độ Short chỉ mở riêng cho phân hệ Phái sinh (VN30F).

---

## 📊 PHẦN 2: KIẾN TRÚC DỮ LIỆU & PIPELINE (DATA ENGINEERING SPEC)

Hệ thống yêu cầu một cơ sở dữ liệu chuỗi thời gian (Time-series) chuẩn hóa cao để phục vụ cho cả render biểu đồ thời gian thực lẫn tính toán backtest ma trận.

### 2.1 Cấu trúc bảng dữ liệu (Schema Design)

#### Bảng 1: `dim_tickers` (Danh mục cổ phiếu)

| Tên trường | Kiểu dữ liệu | Ràng buộc | Mô tả |
| --- | --- | --- | --- |
| `ticker` | `VARCHAR(10)` | `PRIMARY KEY` | Mã chứng khoán (e.g., SSI, FPT, VNM) |
| `exchange` | `VARCHAR(10)` | `NOT NULL` | Sàn giao dịch: HOSE, HNX, UPCOM |
| `market_cap_group` | `VARCHAR(10)` |  | Phân loại: VN30, Midcap, Smallcap |
| `is_active` | `BOOLEAN` | `DEFAULT TRUE` | Trạng thái đang giao dịch hay hủy niêm yết |

#### Bảng 2: `fact_stock_ohlcv_daily` (Dữ liệu giá ngày đã điều chỉnh)

> *Lưu ý: Bắt buộc sử dụng giá đã điều chỉnh (Adjusted Price) cho các trường Open, High, Low, Close để tính toán chỉ báo kỹ thuật.*

| Tên trường | Kiểu dữ liệu | Ràng buộc | Mô tả |
| --- | --- | --- | --- |
| `id` | `BIGSERIAL` | `PRIMARY KEY` | Khóa chính tự tăng |
| `ticker` | `VARCHAR(10)` | `FOREIGN KEY` | Liên kết tới bảng `dim_tickers` |
| `date` | `DATE` | `NOT NULL` | Ngày giao dịch |
| `open` | `NUMERIC(15, 2)` | `NOT NULL` | Giá mở cửa đã điều chỉnh (Adj Open) |
| `high` | `NUMERIC(15, 2)` | `NOT NULL` | Giá cao nhất đã điều chỉnh (Adj High) |
| `low` | `NUMERIC(15, 2)` | `NOT NULL` | Giá thấp nhất đã điều chỉnh (Adj Low) |
| `close` | `NUMERIC(15, 2)` | `NOT NULL` | Giá đóng cửa đã điều chỉnh (Adj Close) |
| `volume` | `BIGINT` | `NOT NULL` | Khối lượng giao dịch khớp lệnh |
| `basic_close` | `NUMERIC(15, 2)` | `NOT NULL` | Giá đóng cửa chưa điều chỉnh (Dùng hiển thị thực tế) |

*Chỉ mục (Indexing Strategy):* Tạo `COMPOSITE INDEX` trên cặp `(ticker, date DESC)` để tối ưu tốc độ truy vấn data nạp vào đồ thị.

### 2.2 Data Pipeline & Data Cleaning Logic

* **Nguồn cấp (Data Ingestion):** Tích hợp thông qua `vnstock` API kết nối tới các endpoint của các công ty chứng khoán (TCBS/SSI) để cập nhật dữ liệu EOD (End of Day) tự động vào 16:00 hàng ngày.
* **Xử lý khoảng trống (Gap Handling):** Đối với các ngày thị trường nghỉ giao dịch (Thứ bảy, Chủ nhật, Ngày lễ), hệ thống tuyệt đối không tự chèn nến (No interpolation). Trục thời gian trên biểu đồ sẽ bỏ qua các ngày này để đảm bảo các chỉ báo như EMA, RSI không bị méo mó.
* **Xử lý cổ phiếu không có thanh khoản:** Nếu một ngày có volume = 0, giá trị OHLC sẽ được gán bằng giá Close của ngày liền trước.

---

## 💻 PHẦN 3: ĐẶC TẢ TÍNH NĂNG CHI TIẾT (FUNCTIONAL REQUIREMENTS)

### 3.1 Phân hệ 1: Manual Bar Replay & Đồ thị tương tác (UI/UX TradingView-like)

#### Luồng nghiệp vụ (Workflow)

1. Người dùng gõ tìm kiếm mã cổ phiếu (e.g., "FPT") và chọn khung thời gian `1D`.
2. Hệ thống tải toàn bộ dữ liệu lịch sử đổ vào thư viện `lightweight-charts`.
3. Người dùng click vào icon **"Bar Replay" (Tua nến)** $\rightarrow$ Chọn một ngày bất kỳ trên biểu đồ (Ký hiệu là $D_{replay}$).
4. Giao diện ngay lập tức ẩn toàn bộ các cây nến từ ngày $D_{replay} + 1$ cho đến thời điểm hiện tại.
5. Thanh điều khiển Replay xuất hiện bao gồm các nút: `Play` (Tự động chạy), `Pause` (Tạm dừng), `Forward` (Nhảy thêm 1 nến), `Speed` (Tốc độ: 0.5s, 1s, 3s, 5s một nến).

```
[Chọn mốc Replay] ──> [Ẩn nến tương lai] ──> [Nhấn Forward] ──> [Hiện thêm 1 nến] ──> [Re-calculate Indicators]

```

#### Các yêu cầu kỹ thuật cho Đồ thị:

* **Hỗ trợ Công cụ vẽ (Drawing Toolbox):** Tích hợp thanh công cụ bên trái màn hình. Cho phép lưu tọa độ vẽ (`x1, y1, x2, y2`) của các đường Trendline, Fibonacci vào `localStorage` của trình duyệt theo từng mã cổ phiếu, tránh bị mất khi reload trang.
* **Tính toán chỉ báo động (Dynamic Indicators Runtime):** Khi người dùng nhấn `Forward` để sinh ra nến mới, các mảng dữ liệu đầu vào của các chỉ báo (RSI, Bollinger Bands, EMA) sẽ được append thêm phần tử mới và tính toán lại giá trị cuối cùng, render trực tiếp lên chart mà không load lại toàn bộ đồ thị.

### 3.2 Phân hệ 2: Giả lập giao dịch thủ công (Paper Trading Panel)

Nằm ngay phía dưới hoặc bên cạnh biểu đồ Replay, cho phép đặt lệnh dựa trên cây nến hiện tại đang hiển thị.

#### Các loại lệnh hỗ trợ:

* **Market Order (Lệnh thị trường):** Khớp ngay lập tức tại giá `Close` của cây nến hiện tại (nếu đang bấm dừng) hoặc giá `Open` của cây nến tiếp theo khi nhấn `Forward`.
* **Limit Order (Lệnh giới hạn):** Người dùng đặt mức giá chờ mua $P_{limit}$. Khi bấm `Forward` qua từng cây nến, hệ thống kiểm tra điều kiện: Nếu $Low_{candle} \le P_{limit} \le High_{candle}$ $\rightarrow$ Kích hoạt khớp lệnh thành công.

#### Logic xử lý chu kỳ thanh toán Việt Nam (T+2 Simulation):

Hệ thống duy trì một danh sách các vị thế (Positions) trong bộ nhớ:

```python
# Cấu trúc dữ liệu quản lý vị thế giả lập
position = {
    "ticker": "SSI",
    "quantity": 1000,
    "entry_price": 35000,
    "entry_date": "2026-06-15", # Ngày T+0
    "available_quantity": 0,    # Khối lượng có thể bán tại phiên hiện tại
    "status": "HOLDING"
}

```

* **Logic cập nhật trạng thái:** Khi dòng thời gian của Replay tiến lên, một hàm kiểm tra lịch trình giao dịch (`check_t_plus_status`) sẽ đếm số phiên giao dịch chính thức (bỏ qua ngày lễ/cuối tuần). Khi số phiên trôi qua kể từ `entry_date` đạt mức quy định, hệ thống sẽ cập nhật:

$$\text{Nếu } \text{Current\_Date} \ge \text{Entry\_Date} + 2 \text{ phiên} \implies \text{available\_quantity} = \text{quantity}$$


* Nếu người dùng cố tình nhấn lệnh `SELL` khi `available_quantity == 0`, hệ thống sẽ chặn và hiển thị thông báo lỗi: *"Cổ phiếu chưa về tài khoản theo quy định T+2"*.

### 3.3 Phân hệ 3: Động cơ Backtest Thuật toán (Systematic Algorithmic Engine)

Phân hệ này dành cho việc thực thi chiến lược tự động bằng Python thông qua một giao diện code editor hoặc tải file chiến lược lên hệ thống.

#### Cú pháp định nghĩa chiến lược (Mô phỏng lớp cấu trúc chuyên nghiệp)

Hệ thống cung cấp một lớp cơ sở `BaseStrategy` để người dùng kế thừa. Giao diện lập trình ứng dụng (API) yêu cầu cấu trúc nghiêm ngặt để tối ưu hóa hiệu năng tính toán:

```python
import pandas as pd
import pandas_ta as ta

class BaseStrategy:
    def __init__(self, data):
        self.data = data  # DataFrame chứa cột: Open, High, Low, Close, Volume
        self.positions = []
        self.cash = 100_000_000  # Vốn khởi tạo mặc định: 100 triệu VND
        self.commission = 0.0025 # Phí + Thuế mặc định: 0.25%

    def init(self):
        """Khởi tạo toàn bộ chỉ báo kỹ thuật dạng Vector hóa (Vectorized) để tối ưu tốc độ"""
        pass

    def next(self, current_index, current_row):
        """Xử lý logic tại từng cây nến (Vòng lặp lịch sử)"""
        pass

```

#### Cơ chế khớp lệnh và tính toán số dư tài sản (Accounting Ledger)

Tại mỗi phiên giao dịch $t$ trong hàm `next`:

* **Khi lệnh BUY thực thi thắng lợi:**
* Số tiền mặt bị trừ: $\text{Cash}_{t} = \text{Cash}_{t-1} - (\text{Quantity} \times \text{Price}_{entry} \times (1 + \text{Commission}))$
* Ghi nhận một bản ghi trạng thái lệnh mua vào sổ cái (Ledger).


* **Khi lệnh SELL thực thi thắng lợi:**
* Kiểm tra điều kiện ràng buộc thời gian mua: $t_{sell} - t_{buy} \ge 2 \text{ phiên}$.
* Số tiền mặt nhận về: $\text{Cash}_{t} = \text{Cash}_{t-1} + (\text{Quantity} \times \text{Price}_{exit} \times (1 - \text{Commission}))$



---

## 📈 PHẦN 4: THỐNG KÊ HIỆU SUẤT & PHÂN TÍCH (ANALYTICS SPEC)

Sau khi kết thúc chu kỳ Backtest thuật toán hoặc kết thúc một chuỗi lệnh Replay thủ công, hệ thống phải xuất một Dashboard báo cáo phân tích tài chính sâu sắc.

### 4.1 Danh mục các chỉ số bắt buộc tính toán (Metrics Blueprint)

| Tên chỉ số | Công thức toán học / Logic xử lý | Ý nghĩa nghiệp vụ |
| --- | --- | --- |
| **Initial Cash** | Khởi tạo ban đầu (e.g., $100,000,000$ VND) | Số vốn ban đầu đưa vào hệ thống. |
| **Final Equity** | $\text{Cash}_{end} + \sum (\text{Quantity}_{holding} \times \text{Price}_{end})$ | Tổng giá trị tài sản ròng tại ngày cuối cùng. |
| **Total Return (%)** | $\left( \frac{\text{Final Equity} - \text{Initial Cash}}{\text{Initial Cash}} \right) \times 100$ | Tỷ suất lợi nhuận tổng thể của chiến lược. |
| **Win Rate (%)** | $\left( \frac{\text{Số lệnh có lợi nhuận}}{\text{Tổng số lệnh đã đóng}} \right) \times 100$ | Xác suất thắng của hệ thống giao dịch. |
| **Profit Factor** | $\frac{\sum \text{Lợi nhuận từ các lệnh thắng}}{\sum \| \text{Thua lỗ từ các lệnh thua} \$ | Hệ số lợi nhuận. Nếu $< 1.0$, chiến lược lỗ. |
| **Max Drawdown (%)** | $\max_{t} \left( \frac{\text{Peak Equity}_t - \text{Equity}_t}{\text{Peak Equity}_t} \right) \times 100$ | Mức sụt giảm tài sản lớn nhất từ đỉnh cao nhất. Đo lường rủi ro chịu đựng phá sản. |
| **Sharpe Ratio** | $\frac{R_p - R_f}{\sigma_p}$ *(với $R_f = 4.5\%$ lãi suất phi rủi ro VN)* | Đo lường hiệu suất sinh lời trên một đơn vị rủi ro biến động tài sản. |

### 4.2 Cấu trúc biểu đồ đầu ra (Output Visualizations)

Dashboard báo cáo kết quả yêu cầu 3 đồ thị trực quan:

1. **Biểu đồ đường Equity Curve (Tài sản ròng):** Vẽ đường biến động tổng tài sản theo thời gian thực hiện backtest, đặt song song đè lên đường biến động của chỉ số VN-Index (Benchmark) để chứng minh chiến lược có "outperform" thị trường chung hay không.
2. **Biểu đồ cột Drawdown Duration:** Hiển thị chi tiết những giai đoạn tài sản bị sụt giảm, giúp người dùng nhận diện chiến lược phải mất bao lâu để phục hồi lại đỉnh cũ.
3. **Biểu đồ phân phối lợi nhuận (Trade Distribution):** Dạng Histogram thể hiện tần suất xuất hiện rải rác của các mức lãi/lỗ của từng lệnh, giúp phát hiện các lệnh thắng đột biến (Outliers) có thể làm sai lệch bản chất hệ thống.

---

## 🏗️ PHẦN 5: KIẾN TRÚC KỸ THUẬT & SƠ ĐỒ LUỒNG (TECHNICAL SPEC)

### 5.1 Technology Stack đề xuất

* **Backend Framework:** FastAPI (Python) - Đảm bảo tốc độ xử lý tính toán cực nhanh, hỗ trợ cơ chế Async phục vụ việc streaming dữ liệu nến cho UI Replay qua kết nối WebSocket.
* **Frontend Framework:** React.js kết hợp TypeScript - Quản lý State chặt chẽ cho biểu đồ vẽ kỹ thuật.
* **Charting Library:** TradingView Lightweight Charts (Bản phân phối mã nguồn mở JavaScript).
* **Database:** PostgreSQL (Lưu trữ metadata, thông tin tài khoản, nhật ký lệnh) + Cài đặt extension **TimescaleDB** để tối ưu hóa truy vấn chuỗi thời gian đối với bảng `fact_stock_ohlcv_daily`.
* **Computation Core:** Thư viện `pandas`, `numpy` và `pandas-ta` được đóng gói trong một Worker chạy ngầm (Celery/Redis) khi thực hiện các tác vụ backtest nặng hoặc chạy tối ưu hóa tham số.

### 5.2 Sơ đồ luồng xử lý lệnh trong Động cơ Backtest (Execution Flow)

```
       [ Cây Nến Lịch Sử Phiên t ]
                   │
                   ▼
       ┌───────────────────────┐
       │   Hàm next() Thực Thi │
       └───────────────────────┘
                   │
         [ Phát Sinh Lệnh Mua? ]
                   │
         ┌─────────┴─────────┐
        YES                  NO
         │                   │
         ▼                   ▼
┌──────────────────┐  [ Phát Sinh Lệnh Bán? ]
│ Kiểm Tra Số Dư   │         │
│ Tiền Mặt (Cash)  │    ┌────┴────┐
└────────┬─────────┘   YES        NO ──> [ Bỏ Qua, Tiến Đến Phiên t+1 ]
         │              │
    (Đủ Điều Kiện)      ▼
         │       ┌────────────────────────┐
         ▼       │ Kiểm Tra Chu Kỳ T+2    │
┌──────────────────┐ │ Khối Lượng Có Sẵn?     │
│ Khớp Lệnh Mua    │ └──────────┬─────────────┘
│ Trừ Cash         │            │
│ Thêm Vị Thế Mới  │       (Đủ Điều Kiện)
└──────────────────┘            │
                                ▼
                        ┌──────────────────┐
                        │ Khớp Lệnh Bán    │
                        │ Cộng Cash        │
                        │ Đóng Vị Thế      │
                        └──────────────────┘

```

---

## 🛡️ PHẦN 6: KẾ HOẠCH PHÁT TRIỂN & CÁC TRƯỜNG HỢP KIỂM THỬ (TEST CASES)

Để đảm bảo hệ thống vận hành đúng chuẩn trước khi bàn giao, đội ngũ phát triển và QA cần tuân thủ nghiêm ngặt bảng kịch bản kiểm thử biên dưới đây.

### Các kịch bản kiểm thử hệ thống cốt lõi (Boundary Test Cases)

| ID | Phân hệ | Tiêu đề Test Case | Các bước thực hiện | Kết quả kỳ vọng (Pass Criteria) |
| --- | --- | --- | --- | --- |
| **TC-001** | Data | Kiểm tra xử lý chia tách cổ tức | Nạp dữ liệu mã cổ phiếu có lịch sử chia thưởng cổ phiếu tỷ lệ 1:1 trong quá khứ. | Giá biểu đồ không bị gãy gáp giảm sâu. Các chỉ báo kỹ thuật như MA tại ngày chia không bị lệch cấu trúc. |
| **TC-002** | Replay | Kiểm tra lỗi Look-ahead | Bật chế độ Bar Replay tại ngày `2025-01-02`. Gọi chỉ báo `ta.ema(length=20)`. | Giá trị chỉ báo tại ngày `2025-01-02` hoàn toàn độc lập, không sử dụng bất kỳ dữ liệu nào của các ngày sau đó. |
| **TC-003** | Engine | Kiểm tra chặn bán T+1 | Thực hiện lệnh `BUY` mã SSI tại phiên Replay thứ Hai. Ngay phiên thứ Ba liền kề bấm lệnh `SELL`. | Hệ thống từ chối lệnh bán, bắn thông báo lỗi vi phạm chu kỳ thanh toán T+2 của thị trường Việt Nam. |
| **TC-004** | Engine | Kiểm tra vi phạm biên độ trần/sàn | Đặt một lệnh mua giới hạn (Limit Order) cổ phiếu HOSE với mức giá cao hơn $10\%$ so với giá đóng cửa phiên trước đó. | Hệ thống từ chối ghi nhận lệnh vào hàng đợi, trạng thái lệnh chuyển thành `REJECTED` do vượt biên độ trần quy định ($7\%$). |
| **TC-005** | Analytics | Kiểm tra trường hợp phá sản (Margin Call/Bankruptcy) | Cấu hình tài khoản ban đầu 10 triệu. Thực hiện chuỗi lệnh thua liên tục cho đến khi tài sản ròng về sát mức 0. | Hệ thống tự động dừng tiến trình backtest, ghi nhận `Max Drawdown = 100%`, xuất báo cáo tại thời điểm phá sản, không bị lỗi treo luồng tính toán (`Division by Zero`). |

---

Tài liệu Spec này định hình khung xương vững chắc cho dự án hệ thống Backtest chứng khoán. Bạn hoàn toàn có thể sử dụng cấu trúc PRD chi tiết này để làm kim chỉ nam thiết kế cơ sở dữ liệu, phân chia task lập trình Backend/Frontend hoặc làm tài liệu bàn giao nghiệp vụ cho các kỹ sư trong đội ngũ phát triển dự án.
