# PRODUCT SPEC — VN Replay Trading Lab

## 1. Tên dự án

Tên sản phẩm:

```text
VN Replay Trading Lab
```

Tên repository đề xuất:

```text
vn-replay-trading-lab
```

## 2. Tầm nhìn sản phẩm

`VN Replay Trading Lab` là một webapp local-first phục vụ cá nhân, dùng để luyện phân tích kỹ thuật chứng khoán Việt Nam thông qua dữ liệu quá khứ.

Sản phẩm cho phép người dùng:

* Import dữ liệu lịch sử từ CafeF.
* Xem biểu đồ nến chứng khoán Việt Nam.
* Replay từng cây nến trong quá khứ giống TradingView Bar Replay.
* Ẩn dữ liệu tương lai để tránh thiên kiến nhìn trước.
* Tự đưa ra quyết định Buy / Sell / Hold / Skip / Add / Reduce / Close.
* Ghi lại lý do ra quyết định, setup giao dịch, stop-loss, target, mức tự tin và lỗi mắc phải.
* Theo dõi position, trade lifecycle, PnL, R-multiple và hiệu suất giao dịch.
* Thống kê năng lực ra quyết định theo mã cổ phiếu, setup, lỗi giao dịch, khung thời gian và bối cảnh thị trường.

Đây không phải là một backtest engine đơn giản. Đây là một hệ thống luyện kỹ năng giao dịch thủ công kết hợp:

```text
TradingView Bar Replay
+ Trading Journal
+ Manual Backtesting
+ Performance Analytics
+ Technical Analysis Workspace
```

## 3. Lý do xây dựng sản phẩm

Người dùng hiện sử dụng TradingView bản free nên gặp các giới hạn:

* Không dùng được nhiều indicator cùng lúc.
* Không replay/backtest linh hoạt nhiều giai đoạn hoặc nhiều khung thời gian.
* Không có hệ thống journal riêng cho thị trường Việt Nam.
* Không thống kê được tỷ lệ thắng/thua theo setup cá nhân.
* Không biết mình thường sai ở đâu khi phân tích kỹ thuật.
* Không có công cụ luyện tập quyết định mua/bán bằng dữ liệu lịch sử chứng khoán Việt Nam.

Sản phẩm cần giúp người dùng biến quá trình “xem chart và cảm nhận” thành dữ liệu có thể phân tích được.

Mục tiêu cuối cùng không chỉ là biết một chiến lược có lời hay không, mà là biết:

```text
Tôi ra quyết định có tốt không?
Tôi thắng ở setup nào?
Tôi thua vì lỗi gì?
Tôi có tuân thủ stop-loss không?
Tôi có hay mua đuổi không?
Tôi có giỏi giao dịch breakout, pullback, nền giá hay bắt đáy không?
Tôi giao dịch tốt hơn trong bối cảnh thị trường nào?
```

## 4. Nguyên tắc sản phẩm

### 4.1. Không làm prototype vứt đi

Dự án phải được thiết kế như một sản phẩm nghiêm túc:

* Có domain model rõ ràng.
* Có database schema sạch.
* Có service layer.
* Có test cho logic quan trọng.
* Có kiến trúc có thể mở rộng.
* Không viết code tạm khiến sau này phải đập đi làm lại.

### 4.2. Triển khai theo vertical slice

Dù thiết kế chuyên nghiệp, việc triển khai vẫn phải chia thành từng lát cắt nhỏ có thể chạy được.

Lát cắt đầu tiên:

```text
CafeF Import
→ Candle Database
→ Replay Session
→ Chart Display
→ Manual Decision
→ Position / Trade
→ Basic Journal
→ Basic Analytics
```

### 4.3. Tuyệt đối không leak dữ liệu tương lai

Đây là nguyên tắc quan trọng nhất.

Trong Replay Mode:

```text
Backend chỉ được trả dữ liệu tới current_index của replay session.
Frontend không được nhận toàn bộ dữ liệu lịch sử rồi tự slice.
```

Sai:

```text
Backend trả toàn bộ candles cho frontend.
Frontend chỉ ẩn phần tương lai.
```

Đúng:

```text
Backend chỉ trả candles từ đầu session đến current_index.
Frontend không có quyền truy cập candles tương lai.
```

Cần có test tự động để đảm bảo rule này.

### 4.4. Tách Decision, Order, Execution, Position, Trade

Không được gộp mọi thứ vào một bảng `orders` đơn giản.

Cần phân biệt:

```text
Decision  = người dùng nghĩ gì và quyết định gì.
Order     = hành động giả lập được đặt ra.
Execution = kết quả khớp lệnh giả lập.
Position  = trạng thái nắm giữ hiện tại.
Trade     = vòng đời giao dịch đã đóng hoặc đang mở.
```

Lý do:

* Một decision có thể sinh ra một hoặc nhiều order.
* Một position có thể được add/reduce nhiều lần.
* Một trade có thể có nhiều execution.
* Journal cần lưu cả suy nghĩ, không chỉ lệnh mua/bán.
* Analytics cần tính đúng PnL, R-multiple, holding period, win/loss.

### 4.5. Ưu tiên đúng logic hơn nhiều tính năng

Không ưu tiên làm nhiều indicator nếu core replay/trade lifecycle chưa đúng.

Thứ tự ưu tiên:

```text
1. Data đúng
2. Replay không leak tương lai
3. Decision được lưu đúng
4. Position/trade tính đúng
5. Journal có ý nghĩa
6. Analytics đúng
7. Chart đẹp
8. Indicator nhiều
9. Strategy automation
```

## 5. Người dùng mục tiêu

Người dùng chính:

```text
Nhà đầu tư cá nhân tại Việt Nam
Có hiểu biết cơ bản về chứng khoán
Muốn luyện phân tích kỹ thuật
Muốn tự đánh giá chất lượng quyết định giao dịch
Muốn dùng dữ liệu chứng khoán Việt Nam
Muốn chạy local cho cá nhân
```

Hiện tại không cần multi-user, không cần public SaaS, không cần subscription, không cần phân quyền phức tạp.

## 6. Phạm vi sản phẩm

### 6.1. Trong phạm vi phiên bản đầu

Phiên bản đầu cần có:

* Import dữ liệu CafeF thủ công.
* Lưu dữ liệu OHLCV vào database.
* Quản lý danh sách mã cổ phiếu.
* Hiển thị chart nến.
* Replay từng cây nến.
* Tạo session replay.
* Ghi decision.
* Giả lập lệnh cơ bản.
* Quản lý position cơ bản.
* Đóng trade.
* Tính PnL.
* Tính win/loss.
* Tính R-multiple nếu có stop-loss.
* Hiển thị journal.
* Hiển thị report cơ bản.

### 6.2. Ngoài phạm vi phiên bản đầu

Chưa làm trong bản đầu:

* Real-time data.
* Tự động đặt lệnh broker.
* Tự động crawl CafeF hàng loạt.
* Login nhiều user.
* Cloud deployment.
* AI khuyến nghị mua/bán.
* Machine learning.
* Intraday data.
* Đồng bộ danh mục MBS/VPS.
* Strategy builder phức tạp.
* Backtest tự động nâng cao.
* Mobile app riêng.

## 7. Nguồn dữ liệu

### 7.1. Nguồn chính ban đầu

Nguồn chính:

```text
CafeF data download
```

Trang nguồn:

```text
https://cafef.vn/du-lieu/du-lieu-download.chn
```

Cách dùng trong phiên bản đầu:

```text
Người dùng tải file dữ liệu từ CafeF thủ công
→ upload file vào app
→ app parse file
→ chuẩn hóa dữ liệu
→ lưu vào database
```

Không ưu tiên crawl tự động trong phiên bản đầu.

### 7.2. Loại file cần hỗ trợ

Importer nên hỗ trợ linh hoạt:

```text
.csv
.txt
.zip chứa .csv hoặc .txt
```

CafeF có thể cung cấp dữ liệu cho AmiBroker/MetaStock. Vì vậy importer cần linh hoạt với các format khác nhau.

### 7.3. Trường dữ liệu tối thiểu

Dữ liệu nến tối thiểu:

```text
symbol
trade_date
open
high
low
close
volume
```

Trường chuẩn hóa trong hệ thống:

```text
symbol
timeframe
trade_date
open
high
low
close
volume
source
adjustment_type
created_at
updated_at
```

### 7.4. Adjusted và unadjusted data

Hệ thống cần phân biệt dữ liệu:

```text
adjusted
unadjusted
unknown
index
```

Lý do:

* Backtest dài hạn nên ưu tiên adjusted data.
* Phân tích chart thực tế có thể cần unadjusted data.
* Index như VNINDEX/VN30 có thể cần đánh dấu riêng.
* Không được trộn adjusted và unadjusted mà không biết.

### 7.5. Timeframe

Phiên bản đầu hỗ trợ:

```text
1D
```

Sau đó có thể thêm:

```text
1W
1M
```

Weekly/Monthly có thể được tạo bằng cách resample từ daily data.

### 7.6. Data quality check

Sau khi import, cần kiểm tra:

* Dòng bị thiếu symbol.
* Dòng bị thiếu ngày.
* Dòng bị thiếu open/high/low/close.
* Giá âm hoặc bằng 0 bất thường.
* Volume âm.
* Dữ liệu trùng.
* Candle có high nhỏ hơn low.
* Open/close nằm ngoài high/low.
* Ngày không parse được.
* Symbol có ký tự bất thường.
* Khoảng ngày bị thiếu quá nhiều.

Importer cần trả summary rõ ràng:

```json
{
  "imported_rows": 10000,
  "skipped_rows": 50,
  "duplicate_rows": 120,
  "symbols_count": 80,
  "start_date": "2020-01-01",
  "end_date": "2026-06-15",
  "warnings": []
}
```

## 8. Kiến trúc kỹ thuật

### 8.1. Tổng quan

Kiến trúc đề xuất:

```text
Frontend
  React + TypeScript + Vite
  TradingView Lightweight Charts
  TanStack Query
  Zustand hoặc Context

Backend
  Python
  FastAPI
  SQLAlchemy
  Pydantic
  Pandas

Database
  SQLite cho local-first
  Thiết kế abstraction để có thể chuyển sang PostgreSQL sau

Analytics
  Pandas
  Custom analytics service

Testing
  pytest cho backend
  Vitest/React Testing Library cho frontend nếu cần
```

### 8.2. Lý do chọn stack

Backend Python vì:

* Xử lý data tài chính thuận tiện.
* Pandas mạnh cho OHLCV.
* Dễ tính indicator.
* Dễ mở rộng sang backtest tự động.

FastAPI vì:

* Nhanh.
* Có OpenAPI docs.
* Dễ dùng với Pydantic.
* Phù hợp local API.

React + TypeScript vì:

* Phù hợp làm chart workspace.
* Dễ mở rộng UI.
* Có thể tích hợp TradingView Lightweight Charts.

SQLite vì:

* Dễ chạy local.
* Không cần cài PostgreSQL ngay.
* Phù hợp cá nhân.
* Dễ backup file database.

Tuy vậy code không được phụ thuộc cứng vào SQLite. Nên dùng SQLAlchemy để sau này đổi sang PostgreSQL.

## 9. Cấu trúc thư mục đề xuất

```text
vn-replay-trading-lab/
├── backend/
│   ├── app/
│   │   ├── main.py
│   │   ├── config.py
│   │   ├── db.py
│   │   ├── dependencies.py
│   │   ├── models/
│   │   │   ├── symbol.py
│   │   │   ├── candle.py
│   │   │   ├── replay_session.py
│   │   │   ├── decision.py
│   │   │   ├── order.py
│   │   │   ├── execution.py
│   │   │   ├── position.py
│   │   │   ├── trade.py
│   │   │   ├── journal_entry.py
│   │   │   ├── indicator_template.py
│   │   │   └── event_log.py
│   │   ├── schemas/
│   │   │   ├── symbol_schema.py
│   │   │   ├── candle_schema.py
│   │   │   ├── import_schema.py
│   │   │   ├── replay_schema.py
│   │   │   ├── decision_schema.py
│   │   │   ├── order_schema.py
│   │   │   ├── trade_schema.py
│   │   │   ├── journal_schema.py
│   │   │   └── analytics_schema.py
│   │   ├── api/
│   │   │   ├── health.py
│   │   │   ├── import_data.py
│   │   │   ├── symbols.py
│   │   │   ├── candles.py
│   │   │   ├── replay.py
│   │   │   ├── decisions.py
│   │   │   ├── orders.py
│   │   │   ├── journal.py
│   │   │   └── analytics.py
│   │   ├── services/
│   │   │   ├── cafef_importer.py
│   │   │   ├── data_quality_service.py
│   │   │   ├── candle_service.py
│   │   │   ├── indicator_service.py
│   │   │   ├── replay_service.py
│   │   │   ├── decision_service.py
│   │   │   ├── execution_service.py
│   │   │   ├── position_service.py
│   │   │   ├── trade_service.py
│   │   │   ├── journal_service.py
│   │   │   └── analytics_service.py
│   │   ├── domain/
│   │   │   ├── enums.py
│   │   │   ├── money.py
│   │   │   └── errors.py
│   │   └── tests/
│   │       ├── test_cafef_importer.py
│   │       ├── test_replay_no_future_leak.py
│   │       ├── test_trade_lifecycle.py
│   │       └── test_analytics.py
│   ├── requirements.txt
│   ├── pyproject.toml
│   └── README.md
│
├── frontend/
│   ├── src/
│   │   ├── api/
│   │   │   ├── client.ts
│   │   │   ├── importApi.ts
│   │   │   ├── replayApi.ts
│   │   │   ├── decisionApi.ts
│   │   │   └── analyticsApi.ts
│   │   ├── components/
│   │   │   ├── chart/
│   │   │   │   ├── CandleChart.tsx
│   │   │   │   ├── VolumePane.tsx
│   │   │   │   ├── IndicatorOverlay.tsx
│   │   │   │   └── TradeMarkers.tsx
│   │   │   ├── replay/
│   │   │   │   ├── ReplayControls.tsx
│   │   │   │   ├── ReplaySessionSelector.tsx
│   │   │   │   └── ReplayStatusBar.tsx
│   │   │   ├── decision/
│   │   │   │   ├── DecisionPanel.tsx
│   │   │   │   ├── OrderTicket.tsx
│   │   │   │   └── SetupSelector.tsx
│   │   │   ├── journal/
│   │   │   │   ├── JournalEditor.tsx
│   │   │   │   └── JournalList.tsx
│   │   │   └── analytics/
│   │   │       ├── PerformanceSummary.tsx
│   │   │       ├── SetupPerformanceTable.tsx
│   │   │       └── TradeList.tsx
│   │   ├── pages/
│   │   │   ├── ImportPage.tsx
│   │   │   ├── DataCenterPage.tsx
│   │   │   ├── ReplayPage.tsx
│   │   │   ├── JournalPage.tsx
│   │   │   ├── AnalyticsPage.tsx
│   │   │   └── SettingsPage.tsx
│   │   ├── store/
│   │   │   ├── replayStore.ts
│   │   │   └── settingsStore.ts
│   │   ├── types/
│   │   │   ├── candle.ts
│   │   │   ├── replay.ts
│   │   │   ├── decision.ts
│   │   │   ├── trade.ts
│   │   │   └── analytics.ts
│   │   ├── App.tsx
│   │   └── main.tsx
│   ├── package.json
│   └── README.md
│
├── data/
│   ├── imports/
│   ├── raw/
│   ├── processed/
│   └── market.db
│
├── docs/
│   ├── PRODUCT_SPEC.md
│   ├── DATA_FORMAT.md
│   ├── API_SPEC.md
│   ├── ROADMAP.md
│   └── DECISIONS.md
│
├── AGENTS.md
├── README.md
└── .gitignore
```

## 10. Domain model

### 10.1. Symbol

Đại diện cho một mã chứng khoán hoặc chỉ số.

Thuộc tính:

```text
symbol
exchange
company_name
sector
industry
asset_type
is_active
```

`asset_type` có thể là:

```text
stock
index
etf
future
unknown
```

### 10.2. Candle

Đại diện cho một cây nến OHLCV.

Thuộc tính:

```text
symbol
timeframe
trade_date
open
high
low
close
volume
source
adjustment_type
```

Unique constraint:

```text
symbol + timeframe + trade_date + adjustment_type
```

### 10.3. ReplaySession

Đại diện cho một phiên luyện tập replay.

Thuộc tính:

```text
id
symbol
timeframe
adjustment_type
start_date
end_date
current_index
initial_cash
current_cash
status
mode
hide_symbol
hide_date
created_at
updated_at
```

`status`:

```text
created
active
paused
completed
archived
```

`mode`:

```text
normal
random
blind_symbol
blind_date
```

### 10.4. Decision

Đại diện cho quyết định của người dùng tại một thời điểm.

Thuộc tính:

```text
session_id
symbol
decision_date
candle_index
action
price
confidence_score
setup_type
market_context
reason
note
mistake_tag
created_at
```

`action`:

```text
BUY
SELL
HOLD
SKIP
ADD
REDUCE
CLOSE
CUT_LOSS
TAKE_PROFIT
```

### 10.5. Order

Đại diện cho lệnh giả lập được tạo từ decision.

Thuộc tính:

```text
session_id
decision_id
symbol
side
order_type
requested_price
quantity
capital_percent
status
created_at
```

`side`:

```text
BUY
SELL
```

`order_type`:

```text
MARKET_AT_CLOSE
MARKET_NEXT_OPEN
LIMIT
CUSTOM_PRICE
```

`status`:

```text
created
executed
cancelled
rejected
```

### 10.6. Execution

Đại diện cho kết quả khớp lệnh giả lập.

Thuộc tính:

```text
order_id
session_id
symbol
execution_date
execution_price
quantity
fee
tax
slippage
gross_amount
net_amount
created_at
```

### 10.7. Position

Đại diện cho vị thế đang nắm giữ.

Thuộc tính:

```text
session_id
symbol
quantity
average_price
total_cost
realized_pnl
unrealized_pnl
status
opened_at
closed_at
```

`status`:

```text
open
closed
```

### 10.8. Trade

Đại diện cho một vòng đời giao dịch.

Một trade bắt đầu khi position được mở và kết thúc khi position đóng hoàn toàn.

Thuộc tính:

```text
session_id
symbol
entry_date
entry_price
exit_date
exit_price
quantity
gross_pnl
net_pnl
pnl_percent
initial_stop_loss
target_price
initial_risk
r_multiple
holding_candles
holding_days
status
result
setup_type
mistake_tag
created_at
updated_at
```

`result`:

```text
win
loss
breakeven
open
```

### 10.9. JournalEntry

Đại diện cho ghi chú phân tích.

Thuộc tính:

```text
session_id
decision_id
trade_id
journal_date
trend_view
market_context
support_resistance
setup_quality
risk_plan
emotion_state
pre_trade_note
post_trade_review
lesson_learned
created_at
updated_at
```

### 10.10. EventLog

Lưu lại các sự kiện quan trọng trong session.

Event types:

```text
SESSION_CREATED
SESSION_STARTED
CANDLE_ADVANCED
CANDLE_REWOUND
DECISION_CREATED
ORDER_CREATED
ORDER_EXECUTED
POSITION_OPENED
POSITION_ADDED
POSITION_REDUCED
POSITION_CLOSED
TRADE_OPENED
TRADE_CLOSED
JOURNAL_CREATED
SESSION_COMPLETED
```

Event log giúp debug, audit và replay lại hành vi người dùng.

## 11. Database schema đề xuất

### 11.1. symbols

```sql
CREATE TABLE symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    exchange TEXT,
    company_name TEXT,
    sector TEXT,
    industry TEXT,
    asset_type TEXT DEFAULT 'stock',
    is_active INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 11.2. candles

```sql
CREATE TABLE candles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL DEFAULT '1D',
    trade_date DATE NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL,
    source TEXT DEFAULT 'cafef',
    adjustment_type TEXT DEFAULT 'unknown',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, timeframe, trade_date, adjustment_type)
);
```

### 11.3. replay_sessions

```sql
CREATE TABLE replay_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL DEFAULT '1D',
    adjustment_type TEXT DEFAULT 'unknown',
    start_date DATE NOT NULL,
    end_date DATE,
    current_index INTEGER NOT NULL DEFAULT 0,
    initial_cash REAL NOT NULL DEFAULT 100000000,
    current_cash REAL NOT NULL DEFAULT 100000000,
    status TEXT NOT NULL DEFAULT 'created',
    mode TEXT DEFAULT 'normal',
    hide_symbol INTEGER DEFAULT 0,
    hide_date INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 11.4. decisions

```sql
CREATE TABLE decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    decision_date DATE NOT NULL,
    candle_index INTEGER NOT NULL,
    action TEXT NOT NULL,
    price REAL,
    confidence_score INTEGER,
    setup_type TEXT,
    market_context TEXT,
    reason TEXT,
    note TEXT,
    mistake_tag TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id)
);
```

### 11.5. orders

```sql
CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    decision_id INTEGER,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL DEFAULT 'MARKET_AT_CLOSE',
    requested_price REAL,
    quantity INTEGER,
    capital_percent REAL,
    status TEXT NOT NULL DEFAULT 'created',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id),
    FOREIGN KEY(decision_id) REFERENCES decisions(id)
);
```

### 11.6. executions

```sql
CREATE TABLE executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    session_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    execution_date DATE NOT NULL,
    execution_price REAL NOT NULL,
    quantity INTEGER NOT NULL,
    fee REAL DEFAULT 0,
    tax REAL DEFAULT 0,
    slippage REAL DEFAULT 0,
    gross_amount REAL NOT NULL,
    net_amount REAL NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(order_id) REFERENCES orders(id),
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id)
);
```

### 11.7. positions

```sql
CREATE TABLE positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 0,
    average_price REAL,
    total_cost REAL DEFAULT 0,
    realized_pnl REAL DEFAULT 0,
    unrealized_pnl REAL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'open',
    opened_at DATE,
    closed_at DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id)
);
```

### 11.8. trades

```sql
CREATE TABLE trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    entry_date DATE NOT NULL,
    entry_price REAL NOT NULL,
    exit_date DATE,
    exit_price REAL,
    quantity INTEGER NOT NULL,
    gross_pnl REAL,
    net_pnl REAL,
    pnl_percent REAL,
    initial_stop_loss REAL,
    target_price REAL,
    initial_risk REAL,
    r_multiple REAL,
    holding_candles INTEGER,
    holding_days INTEGER,
    status TEXT NOT NULL DEFAULT 'open',
    result TEXT DEFAULT 'open',
    setup_type TEXT,
    mistake_tag TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id)
);
```

### 11.9. journal_entries

```sql
CREATE TABLE journal_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    decision_id INTEGER,
    trade_id INTEGER,
    journal_date DATE NOT NULL,
    trend_view TEXT,
    market_context TEXT,
    support_resistance TEXT,
    setup_quality TEXT,
    risk_plan TEXT,
    emotion_state TEXT,
    pre_trade_note TEXT,
    post_trade_review TEXT,
    lesson_learned TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES replay_sessions(id),
    FOREIGN KEY(decision_id) REFERENCES decisions(id),
    FOREIGN KEY(trade_id) REFERENCES trades(id)
);
```

### 11.10. event_logs

```sql
CREATE TABLE event_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER,
    event_type TEXT NOT NULL,
    entity_type TEXT,
    entity_id INTEGER,
    payload_json TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## 12. API specification

### 12.1. Health

```http
GET /api/health
```

Response:

```json
{
  "status": "ok"
}
```

### 12.2. Import CafeF data

```http
POST /api/import/cafef
```

Request:

```text
multipart/form-data
file: csv/txt/zip
adjustment_type: adjusted | unadjusted | index | unknown
```

Response:

```json
{
  "imported_rows": 12000,
  "skipped_rows": 50,
  "duplicate_rows": 300,
  "symbols_count": 95,
  "start_date": "2020-01-01",
  "end_date": "2026-06-15",
  "warnings": []
}
```

### 12.3. List symbols

```http
GET /api/symbols
```

Query optional:

```text
asset_type
exchange
search
```

Response:

```json
[
  {
    "symbol": "VCI",
    "exchange": "HOSE",
    "company_name": null,
    "sector": null,
    "industry": null,
    "asset_type": "stock"
  }
]
```

### 12.4. Get candle summary

```http
GET /api/candles/summary
```

Response:

```json
{
  "symbols_count": 100,
  "candles_count": 500000,
  "start_date": "2010-01-01",
  "end_date": "2026-06-15"
}
```

### 12.5. Create replay session

```http
POST /api/replay/sessions
```

Request:

```json
{
  "symbol": "VCI",
  "timeframe": "1D",
  "adjustment_type": "adjusted",
  "start_date": "2022-01-01",
  "end_date": "2024-12-31",
  "initial_cash": 100000000,
  "mode": "normal"
}
```

Response:

```json
{
  "id": 1,
  "symbol": "VCI",
  "timeframe": "1D",
  "current_index": 0,
  "status": "active"
}
```

### 12.6. Get visible candles

```http
GET /api/replay/sessions/{session_id}/candles
```

Important:

```text
This endpoint must only return candles up to current_index.
It must not return future candles.
```

Response:

```json
{
  "session_id": 1,
  "current_index": 120,
  "candles": [
    {
      "time": "2022-01-01",
      "open": 25000,
      "high": 26000,
      "low": 24800,
      "close": 25500,
      "volume": 1000000
    }
  ]
}
```

### 12.7. Next candle

```http
POST /api/replay/sessions/{session_id}/next
```

Request optional:

```json
{
  "steps": 1
}
```

Response:

```json
{
  "session_id": 1,
  "current_index": 121,
  "status": "active"
}
```

### 12.8. Previous candle

```http
POST /api/replay/sessions/{session_id}/previous
```

Request optional:

```json
{
  "steps": 1
}
```

Response:

```json
{
  "session_id": 1,
  "current_index": 120,
  "status": "active"
}
```

### 12.9. Create decision

```http
POST /api/replay/sessions/{session_id}/decisions
```

Request:

```json
{
  "action": "BUY",
  "price": 25000,
  "quantity": 1000,
  "order_type": "MARKET_AT_CLOSE",
  "stop_loss": 23800,
  "target_price": 28000,
  "setup_type": "Breakout",
  "confidence_score": 4,
  "market_context": "VNINDEX tích cực",
  "reason": "Breakout nền giá kèm volume tăng",
  "note": "Giá vượt kháng cự ngắn hạn",
  "mistake_tag": null
}
```

Response:

```json
{
  "decision_id": 10,
  "order_id": 8,
  "execution_id": 8,
  "position_id": 3,
  "trade_id": 2
}
```

### 12.10. Get session decisions

```http
GET /api/replay/sessions/{session_id}/decisions
```

### 12.11. Get open position

```http
GET /api/replay/sessions/{session_id}/position
```

### 12.12. Get session trades

```http
GET /api/replay/sessions/{session_id}/trades
```

### 12.13. Create journal entry

```http
POST /api/replay/sessions/{session_id}/journal
```

Request:

```json
{
  "decision_id": 10,
  "trade_id": 2,
  "trend_view": "Uptrend",
  "market_context": "VNINDEX trên MA20",
  "support_resistance": "Kháng cự 25.000 vừa bị phá",
  "setup_quality": "Good",
  "risk_plan": "Stop dưới đáy gần nhất",
  "emotion_state": "Calm",
  "pre_trade_note": "Mua theo breakout",
  "post_trade_review": null,
  "lesson_learned": null
}
```

### 12.14. Get analytics report

```http
GET /api/replay/sessions/{session_id}/analytics
```

Response:

```json
{
  "total_trades": 20,
  "closed_trades": 18,
  "open_trades": 2,
  "winning_trades": 10,
  "losing_trades": 8,
  "win_rate": 55.56,
  "total_net_pnl": 12000000,
  "total_pnl_percent": 12.0,
  "average_win": 2500000,
  "average_loss": -1200000,
  "profit_factor": 2.1,
  "average_r": 0.45,
  "expectancy": 500000,
  "best_trade": {},
  "worst_trade": {},
  "setup_performance": [],
  "mistake_performance": []
}
```

## 13. Replay rules

### 13.1. Session creation

Khi tạo session:

1. Load candles theo symbol/timeframe/adjustment_type/date range.
2. Sort tăng dần theo trade_date.
3. Set current_index ban đầu.
4. Nên có warmup candles để indicator có dữ liệu.

Ví dụ:

```text
Nếu start_date = 2022-01-01
Cần MA200
Thì backend có thể load thêm 250 cây nến trước start_date làm warmup
Nhưng UI chỉ nên phân biệt visible trading area và warmup area nếu cần
```

### 13.2. Next candle

Khi bấm Next:

```text
current_index = min(current_index + steps, max_index)
```

Nếu tới cuối:

```text
status = completed
```

### 13.3. Previous candle

Khi bấm Previous:

```text
current_index = max(current_index - steps, 0)
```

Cần cân nhắc:

* Nếu đã đặt decision ở tương lai rồi lùi lại thì xử lý thế nào?
* Phiên bản đầu có thể cho phép lùi để xem lại, nhưng không được xóa decision.
* Có thể cảnh báo nếu lùi trước thời điểm đã đặt lệnh.

### 13.4. Random training mode

Giai đoạn sau:

```text
App random symbol và start_date
Có thể ẩn symbol
Có thể ẩn date
Chỉ hiển thị chart và volume
```

Mục tiêu là tránh thiên kiến.

## 14. Execution rules

### 14.1. Order type

Phiên bản đầu hỗ trợ:

```text
MARKET_AT_CLOSE
CUSTOM_PRICE
```

Sau đó thêm:

```text
MARKET_NEXT_OPEN
LIMIT
STOP
```

### 14.2. Giá khớp mặc định

MVP professional slice:

```text
Nếu action là BUY/SELL và người dùng không nhập price:
Dùng close của current candle.
```

### 14.3. Phí và thuế

Default fee settings:

```text
buy_fee_rate = 0.0015
sell_fee_rate = 0.0015
sell_tax_rate = 0.001
slippage_rate = 0
```

Cần đưa vào settings sau này.

### 14.4. Position update

Khi BUY:

* Nếu chưa có position, mở position.
* Nếu đã có position, tăng quantity và tính lại average_price.
* Tạo hoặc cập nhật trade đang mở.

Khi SELL:

* Nếu chưa có position, reject hoặc báo lỗi.
* Nếu sell quantity nhỏ hơn position quantity, reduce position.
* Nếu sell quantity bằng position quantity, close position và close trade.
* Nếu sell quantity lớn hơn position quantity, reject.

### 14.5. Partial sell

Cần hỗ trợ reduce position.

Với phiên bản đầu, có thể đơn giản:

* Một trade vẫn open nếu chỉ bán một phần.
* Realized PnL cập nhật ở position.
* Trade chỉ close khi quantity về 0.

## 15. Risk and R-multiple

### 15.1. Initial risk

Khi BUY có stop_loss:

```text
initial_risk_per_share = entry_price - stop_loss
initial_risk = initial_risk_per_share * quantity
```

Nếu không có stop_loss:

```text
initial_risk = null
r_multiple = null
```

### 15.2. R-multiple

Khi trade đóng:

```text
r_multiple = net_pnl / initial_risk
```

R-multiple là chỉ số quan trọng hơn % lãi/lỗ vì nó đo chất lượng quản trị rủi ro.

### 15.3. Expectancy

```text
expectancy = win_rate * average_win + loss_rate * average_loss
```

Hoặc theo R:

```text
expectancy_r = win_rate * average_win_r + loss_rate * average_loss_r
```

## 16. Indicator specification

### 16.1. Indicator đầu tiên

Cần hỗ trợ:

```text
SMA20
SMA50
SMA200
EMA20
RSI14
Volume MA20
```

### 16.2. Indicator giai đoạn sau

Thêm:

```text
MACD
Bollinger Bands
ATR
ADX
Ichimoku
Donchian Channel
Relative Strength vs VNINDEX
```

### 16.3. Indicator architecture

Không nên hard-code toàn bộ trong frontend.

Backend hoặc frontend có thể có indicator engine, nhưng cần tổ chức rõ:

```text
IndicatorDefinition
IndicatorParameter
IndicatorResult
IndicatorTemplate
```

Phiên bản đầu có thể tính indicator ở backend bằng Pandas hoặc tính ở frontend nếu đơn giản. Tuy nhiên cần thống nhất format trả về.

## 17. Frontend product pages

### 17.1. Data Center Page

Chức năng:

* Upload file CafeF.
* Chọn adjustment_type.
* Preview dữ liệu.
* Import data.
* Xem số mã đã import.
* Xem số candles.
* Xem range ngày.
* Xem warning data quality.

### 17.2. Replay Page

Layout đề xuất:

```text
Top bar:
  Symbol selector
  Timeframe selector
  Date range
  Create session
  Session status

Main area:
  Candlestick chart
  Volume pane
  Indicator overlays
  Buy/Sell markers

Right panel:
  Decision panel
  Position summary
  Order ticket
  Journal quick note

Bottom panel:
  Replay controls
  Trade log
  Decision log
```

Controls:

```text
Previous
Next
Next 5
Play
Pause
Reset
Complete session
```

Decision buttons:

```text
Buy
Sell
Hold
Skip
Add
Reduce
Close
Cut Loss
Take Profit
```

### 17.3. Journal Page

Chức năng:

* Xem tất cả journal entries.
* Filter theo symbol.
* Filter theo setup_type.
* Filter theo mistake_tag.
* Filter theo result.
* Xem lại decision tại candle cụ thể.
* Ghi post-trade review.
* Ghi lesson learned.

### 17.4. Analytics Page

Chức năng:

* Tổng số trade.
* Win rate.
* Total PnL.
* Profit factor.
* Average R.
* Expectancy.
* Max drawdown.
* Best trade.
* Worst trade.
* Performance by setup.
* Performance by mistake tag.
* Performance by symbol.
* Performance by timeframe.

### 17.5. Settings Page

Chức năng:

* Fee profile.
* Tax profile.
* Slippage.
* Default capital.
* Default order type.
* Indicator templates.
* Setup taxonomy.
* Mistake taxonomy.

## 18. Analytics specification

### 18.1. Basic metrics

```text
total_trades
closed_trades
open_trades
winning_trades
losing_trades
breakeven_trades
win_rate
loss_rate
total_gross_pnl
total_net_pnl
average_win
average_loss
largest_win
largest_loss
profit_factor
average_holding_days
average_holding_candles
```

### 18.2. Risk metrics

```text
average_r
median_r
total_r
expectancy_r
max_drawdown
payoff_ratio
risk_reward_planned
risk_reward_actual
```

### 18.3. Setup analytics

Group by `setup_type`:

```text
trade_count
win_rate
average_r
total_pnl
average_pnl
best_trade
worst_trade
```

### 18.4. Mistake analytics

Group by `mistake_tag`:

```text
count
total_loss
average_loss
related_setups
notes
```

### 18.5. Symbol analytics

Group by `symbol`:

```text
trade_count
win_rate
average_r
total_pnl
```

## 19. Test requirements

Backend tests bắt buộc:

### 19.1. Importer tests

* Parse CSV CafeF-like.
* Parse TXT CafeF-like.
* Parse ZIP containing TXT/CSV.
* Detect missing columns.
* Deduplicate candles.
* Reject invalid OHLC data.

### 19.2. Replay tests

* Create session.
* Get visible candles.
* Next increases current_index.
* Previous decreases current_index.
* Endpoint never returns candles after current_index.
* Completed session cannot advance beyond max index.

### 19.3. Execution tests

* Buy opens position.
* Add increases quantity and updates average price.
* Sell reduces position.
* Sell all closes position.
* Sell more than position is rejected.
* Fee and tax are calculated.
* PnL is calculated correctly.

### 19.4. Trade tests

* Trade opens on first buy.
* Trade closes when position quantity becomes zero.
* Holding days/candles calculated.
* R-multiple calculated if stop-loss exists.
* R-multiple null if stop-loss missing.

### 19.5. Analytics tests

* Win rate.
* Profit factor.
* Average R.
* Expectancy.
* Setup performance.

## 20. Acceptance criteria for first professional slice

Slice đầu tiên được coi là hoàn thành khi:

1. App chạy local được.
2. Có backend FastAPI.
3. Có frontend React.
4. Có SQLite database.
5. Import được file CafeF dạng CSV/TXT/ZIP cơ bản.
6. Lưu được candles.
7. List được symbols.
8. Tạo được replay session.
9. Chart hiển thị nến tới current_index.
10. Backend không trả dữ liệu tương lai.
11. Next Candle hoạt động.
12. Previous Candle hoạt động.
13. Tạo được decision BUY/SELL/HOLD/SKIP.
14. BUY tạo order, execution, position, trade.
15. SELL cập nhật position/trade.
16. Trade đóng tính được PnL.
17. Có journal note cơ bản.
18. Có report cơ bản.
19. Có test cho no-future-leak.
20. Có README hướng dẫn chạy local.

## 21. Roadmap

### Phase 1 — Core foundation

* Project structure.
* Backend models.
* Database.
* CafeF importer.
* Replay service.
* No-future-leak test.

### Phase 2 — Chart replay

* Frontend React.
* Candlestick chart.
* Volume.
* Replay controls.
* Visible candles only.

### Phase 3 — Manual trading simulator

* Decision panel.
* Orders.
* Executions.
* Positions.
* Trades.
* PnL.

### Phase 4 — Journal

* Journal entry.
* Setup tag.
* Mistake tag.
* Confidence score.
* Review notes.

### Phase 5 — Analytics

* Basic performance report.
* R-multiple.
* Expectancy.
* Setup performance.
* Mistake performance.

### Phase 6 — Indicator workspace

* Multiple indicators.
* Indicator settings.
* Indicator templates.
* Multi-pane chart.

### Phase 7 — Multi-timeframe

* Daily/weekly/monthly.
* Resample.
* Multi-timeframe analysis layout.

### Phase 8 — Strategy backtest lab

* Rule-based strategies.
* Compare manual decisions with automated strategy.
* Parameter testing.

### Phase 9 — QMV method integration

* VNINDEX regime.
* Sector strength.
* Stock ranking.
* Dòng tiền classification.
* Custom setup scoring.
* QMV-style trading checklist.

## 22. Coding principles

* Business logic must not live in API route handlers.
* Use service classes/functions.
* Use type hints.
* Use Pydantic schemas for request/response.
* Use SQLAlchemy models.
* Use clear enum values.
* Avoid magic strings where possible.
* Write tests for critical rules.
* Keep functions small.
* Prefer explicit error messages.
* Do not hide exceptions silently.
* Do not add external paid services.
* Do not require cloud to run.
* Do not add authentication unless needed later.
* Do not over-engineer infrastructure too early.

## 23. Local run requirements

Backend:

```bash
cd backend
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app.main:app --reload
```

Frontend:

```bash
cd frontend
npm install
npm run dev
```

Database:

```text
Default database path:
data/market.db
```

## 24. Documentation requirements

Cần có:

```text
README.md
docs/PRODUCT_SPEC.md
docs/DATA_FORMAT.md
docs/API_SPEC.md
docs/ROADMAP.md
AGENTS.md
```

README cần giải thích:

* Dự án là gì.
* Cách chạy backend.
* Cách chạy frontend.
* Cách import dữ liệu CafeF.
* Cách tạo replay session.
* Cách chạy test.
* Các giới hạn hiện tại.

## 25. Definition of Done

Một task chỉ được coi là xong khi:

* Code chạy được.
* Không phá API cũ nếu không có lý do.
* Có error handling cơ bản.
* Có test nếu là business logic quan trọng.
* Có cập nhật README hoặc docs nếu thay đổi hành vi.
* Không leak future candles trong replay mode.
* Không hard-code đường dẫn máy cá nhân.
* Không dùng secret/token.
* Không gửi dữ liệu ra ngoài.
