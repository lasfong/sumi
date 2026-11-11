# SUMI Money Flow Blackbox V1 — MASTER SPEC — Data-Ready v3

> Prompt Doraemon nằm riêng ở `09_DORAEMON_AUDIT_PROMPT.md`.


---

# 00 — Executive Decision

## 1. Có phải chờ data rồi mới DEV BB không?

**Không.**

DEV có thể dựng ngay:

- data contracts;
- source adapters;
- raw/normalized database;
- BB calculator;
- SUMI-420 aggregator;
- API/output schema;
- backfill jobs;
- realtime collector interface;
- validation framework.

Khi data True Flow có, chỉ nạp vào normalized store rồi calculator chạy.

## 2. Solution lý tưởng

Dataset tối giản:

```text
date
symbol
active_buy_value
active_sell_value
```

Core:

```text
BB_H = 100 * SUM(active_buy_value, H)
             / SUM(active_buy_value + active_sell_value, H)
```

`H = 3,5,10,20,50,200`.

## 3. Fallback ladder

```text
A  ACTIVE_TRADE_FLOW_VALUE
   ↓
B  ACTIVE_TRADE_FLOW_VOLUME
   ↓
C  ACTIVE_VOLUME_VALUE_ESTIMATE
   ↓
D  CLASSIFIED_TRADE_FLOW (trade + side/quote)
   ↓
E  OHLCV_PROXY
```

Không đổi core UI/API; đổi `flow_method`.

## 4. Market definition

`SUMI-420` = khoảng 420 mã Sumi quan tâm nhất.

Không cần giải bài toán tất cả mã niêm yết lịch sử.
Universe chỉ cần version hóa khi danh sách thay đổi.

## 5. Vấn đề còn lại

Chủ yếu là **Data Acquisition**:

1. Historical Active Buy/Sell có không?
2. Bao nhiêu năm?
3. Bao phủ 420 mã?
4. Volume hay Value?
5. Có Unknown không?
6. License có cho phép automated collection/persistence không?
7. Nếu không có history, realtime source nào đủ ổn định để tự collect?

## 6. Ba luồng chạy song song

### DEV
Dựng framework ngay.

### Data Research
Audit Doraemon, SSI, FireAnt, Vnstock/Vnstock Data, KBS/VCI/CafeF.

### Data Collection
Nếu có realtime Active Buy/Sell hợp lệ nhưng không có history: **bắt đầu collect ngay**.

Sau 3/5/20/50/200 phiên tương ứng sẽ tự có True Flow BB cho từng horizon.


---

# 01 — Target Architecture

## Nguyên tắc

BB Engine phải **data-source agnostic**.

Không thiết kế:

```text
BBEngine -> FireAnt
```

Mà thiết kế:

```text
Source Adapters
      ↓
Raw Store
      ↓
Normalization
      ↓
Canonical Flow Store
      ↓
BB Engine
      ↓
Symbol BB / SUMI-420 BB / Breadth
```

## Adapter capability metadata

```text
source_name
supports_ohlcv_history
supports_active_flow_history
supports_active_flow_realtime
supports_trade_side
supports_trade_history
supports_order_book
supports_foreign_flow
supports_prop_flow
history_start_date
rate_limit
license_mode
```

## Conceptual adapter functions

```text
fetch_ohlcv(symbol, start, end)
fetch_active_flow_daily(symbol, start, end)
stream_active_trades(symbols)
fetch_trades(symbol, date)
fetch_order_book(symbol)
health_check()
```

Không hỗ trợ thì trả `UNSUPPORTED`; không giả lập field.

## Allowed flow methods

```text
ACTIVE_TRADE_FLOW_VALUE
ACTIVE_TRADE_FLOW_VOLUME
ACTIVE_VOLUME_VALUE_ESTIMATE
CLASSIFIED_TRADE_FLOW
OHLCV_PROXY
```

Mỗi output bắt buộc có:

```text
flow_method
source_id
method_version
quality_score
```

## Không splice method

Ví dụ 2005–2026 có Proxy, từ 2026 có True Flow: backend phải giữ ranh giới method.

## Build-before-data

Phải có:

```text
MockActiveFlowAdapter
CsvActiveFlowAdapter
DbActiveFlowAdapter
```

để integration-test toàn pipeline trước khi vendor thật sẵn sàng.


---

# 02 — Data Requirements & Contracts

## 1. Minimum ideal dataset

```text
date
symbol
active_buy_value
active_sell_value
```

Optional nhưng nên có:

```text
active_buy_volume
active_sell_volume
unknown_value
unknown_volume
total_matched_value
total_matched_volume
source
quality_flags
```

## 2. Vì sao OHLCV không đủ cho True Flow?

OHLCV chỉ giữ giá và tổng volume của cả phiên.
Nó không còn biết trade nào do buyer chủ động đánh Ask hay seller chủ động đập Bid.

Hai ngày có thể đều:

```text
Close +1%
Volume = 10 triệu
Value = 1.000 tỷ
```

nhưng:

```text
Day A: Active Buy 850B / Active Sell 150B -> BB ~85
Day B: Active Buy 350B / Active Sell 650B -> BB ~35
```

Thông tin aggressor side đã mất trong candle.

## 3. Raw trade thay thế

Nếu vendor không có daily aggregate:

```text
trading_date
timestamp
symbol
price
volume
side = BUY|SELL|UNKNOWN
```

Aggregate:

```text
trade_value = price * volume
active_buy_value  = SUM(trade_value WHERE side=BUY)
active_sell_value = SUM(trade_value WHERE side=SELL)
```

## 4. Nếu trade không có side

Cần synchronized:

```text
trade timestamp/price/volume
best_bid
best_ask
quote timestamp
```

để research classifier. Đây là fallback thấp hơn vendor Side vì timestamp/inside-spread/auction phức tạp.

## 5. Active Volume vs Active Value

### Symbol BB
Active Buy/Sell Volume đủ để đo dominance theo volume:

```text
BBV_H = 100 * BuyVol_H / (BuyVol_H + SellVol_H)
```

### SUMI-420
Prefer Value.
Nếu chỉ có volume:

```text
estimated_buy_value  = active_buy_volume  * representative_price
estimated_sell_value = active_sell_volume * representative_price
```

Representative price priority:
1. VWAP/avg matched price;
2. matched value / matched volume;
3. Typical Price.

Method: `ACTIVE_VOLUME_VALUE_ESTIMATE`.

## 6. Unknown handling

```text
classified = buy + sell
coverage = classified / total
```

Unknown không được tự gán BUY/SELL.

## 7. Canonical normalized contract

```text
active_flow_daily
-----------------
trade_date
symbol
active_buy_value
active_sell_value
active_buy_volume
active_sell_volume
unknown_value
unknown_volume
total_matched_value
total_matched_volume
flow_method
source_id
coverage_ratio
quality_score
quality_flags
ingested_at
```


---

# 03 — Data Source Research — snapshot 2026-09-12

> Đây là candidate inventory. Access, retention, cost và license phải audit trước production.

## 1. SSI FastConnect Data — ưu tiên cho forward collection

Official `X-TRADE` streaming hiện mô tả:

```text
TradingDate
Time
Symbol
LastPrice
LastVol
TotalVal
TotalVol
TradingSession
Side
```

`Side = BU / SD / Unknown`.

Nếu access được, Sumi có thể tự aggregate Active Buy/Sell Value từ hôm nay.

### Cần audit
- access/free/fee;
- API registration;
- historical endpoint hay chỉ realtime;
- reconnect/replay;
- SUMI-420 coverage;
- ATO/ATC semantics;
- Unknown rate;
- license/persistence.

**Assessment:** HIGH PRIORITY forward collector.

Docs:
https://guide.ssi.com.vn/ssi-products/tieng-viet/fastconnect-data/du-lieu-streaming
https://guide.ssi.com.vn/ssi-products/change-log

## 2. FireAnt — ưu tiên cho historical daily Active Buy/Sell

MetaKit docs mô tả `_AC` Daily:

```text
O = khối lượng bán chủ động
C = khối lượng mua chủ động
```

FireAnt Big Trade docs cũng mô tả active buy/sell theo volume hoặc value ở indicator.

### Cần audit
- `_AC` history bao nhiêu năm;
- 420 coverage;
- export/API hay MetaKit/AmiBroker only;
- historical value hay chỉ volume;
- package/cost;
- batchability;
- license cho persistence/internal DB;
- khả năng automation qua UiPath nếu cần.

**Assessment:** HIGH PRIORITY history research.

Docs:
https://help.fireant.vn/fireant-metakit/cac-loai-du-lieu/
https://help.fireant.vn/fireant-web/thu-vien-chi-so/ky-thuat/big-trade/

## 3. Vnstock Data — candidate rất đáng kiểm tra trước RPA

Docs hiện mô tả:

```text
Insights().equity(symbol).order_flow()
```

với:

```text
active_buy_volume
active_sell_volume
unknown_volume
```

và `order_flow_history()` với:

```text
active_buy_value
active_sell_value
active_buy_volume
active_sell_volume
```

Đây gần như đúng dataset BB cần.

Nhưng `vnstock_data` là bản Sponsor/extended, không phải free community hoàn toàn; software license cũng không tự cấp quyền data upstream.

### Cần audit
- upstream source;
- start date;
- 420 coverage;
- retention;
- Sponsor tier;
- rate limits;
- internal persistence rights;
- có endpoint tương đương ở source mà Doraemon đã dùng không.

**Assessment:** VERY HIGH PRIORITY.

Docs:
https://vnstocks.com/docs/vnstock-data/cau-truc-du-lieu/insights
https://vnstocks.com/docs/vnstock-data/gioi-thieu-vnstock-data
https://www.vnstocks.com/onboard/giay-phep-su-dung

## 4. CafeF order_stats — không phải True Flow

Các field như:

```text
buy_orders
sell_orders
buy_volume
sell_volume
```

được mô tả là thống kê đặt lệnh. Không đồng nghĩa executed active buy/sell.

Có thể giữ làm feature supply/demand phụ, không dùng làm core True Flow.

Docs:
https://www.vnstocks.com/docs/vnstock-data/du-lieu-giao-dich

## 5. KBS / VCI / nguồn Doraemon

Phải audit raw response/code thực tế:
- có `side` không;
- có active flow hidden/insight endpoint không;
- history không;
- field semantics.

Không suy luận chỉ từ tên library.

## 6. Priority order

```text
P0  Doraemon existing sources
P0  vnstock_data order_flow_history
P0  SSI FastConnect X-TRADE forward stream
P1  FireAnt _AC historical
P1  FireAnt export/MetaKit/Excel
P2  raw trades + quote classification
P3  RPA public pages
P4  paid vendor feed
```


---

# 04 — Collection & RPA Strategy

## 1. Nếu không có historical True Flow

Bắt đầu tự collect **ngay từ hôm nay**.

- ngày 3 có BB03;
- ngày 5 có BB05;
- ngày 20 có BB20;
- ngày 50 có BB50;
- ngày 200 có BB200.

Lịch sử cũ tiếp tục dùng `OHLCV_PROXY`.

## 2. Acquisition order

### Level 1 — Official API/stream
Ưu tiên nhất. Ví dụ SSI X-TRADE Side.

### Level 2 — Documented library/API
Ví dụ vnstock_data nếu license/source phù hợp.

### Level 3 — Export/Excel/MetaKit automation
Nếu vendor cho export nhưng API không phù hợp.

### Level 4 — Browser RPA
UiPath/Playwright chỉ khi không có API/export tốt hơn và usage được phép.

### Level 5 — Paid feed
Nếu tổng cost/risk tốt hơn tự duy trì RPA.

## 3. Tại sao RPA không phải lựa chọn đầu tiên?

- UI đổi dễ hỏng;
- session/login/2FA;
- rate limit/anti-bot;
- 420 mã mỗi ngày;
- completeness khó chứng minh;
- ToS/license;
- debug/maintenance tốn thời gian.

RPA phù hợp hơn cho **daily EOD snapshot**, không phù hợp tick stream.

## 4. UiPath daily collector pattern

```text
Sau giờ đóng cửa
  ↓
Open authenticated/public source
  ↓
Loop SUMI-420
  ↓
Read/export Active Buy/Sell
  ↓
Save RAW capture
  ↓
Normalize
  ↓
Completeness checks
  ↓
Retry failures
  ↓
Daily collection report
```

Mandatory:

```text
source_timestamp
capture_timestamp
source_version
record_hash
symbol/date idempotency
missing-symbol report
retry count
quality flags
```

Nếu website cho CSV/Excel export, automate export thay vì scrape DOM.

## 5. Realtime collector pattern

```text
stream event
   ↓
validate
   ↓
persist raw
   ↓
deduplicate
   ↓
aggregate intraday
   ↓
EOD reconcile
   ↓
active_flow_daily
```

Raw record nên có:

```text
source
received_at
trading_date
event_time
symbol
last_price
last_volume
total_volume
total_value
side
trading_session
raw_payload_hash
raw_payload
```

## 6. Reliability

Collector phải có:
- reconnect;
- heartbeat;
- gap detection;
- duplicate handling;
- source outage alert;
- EOD reconciliation;
- reprocess from raw.

## 7. Hybrid timeline

```text
Old history       : OHLCV_PROXY
Self-collected era: ACTIVE_TRADE_FLOW_VALUE
```

Backend giữ method boundary.

## 8. Cost decision

So sánh:

```text
annual vendor cost
vs RPA maintenance
vs data loss risk
vs license risk
vs engineering complexity
```

Feed trả phí có thể rẻ hơn RPA production nếu RPA cần bảo trì thường xuyên.


---

# 05 — Database & Pipeline Spec

## Tables

### data_source
```text
source_id PK
source_name
source_type
access_type
license_notes
is_active
created_at
```

### sumi_universe_version
```text
universe_version
effective_from
effective_to
description
```

### sumi_universe_member
```text
universe_version
symbol
is_active
```

### active_trade_raw
```text
id
source_id
trading_date
event_time
symbol
price
volume
trade_value
side
session
raw_payload
payload_hash
received_at
quality_flags
```

### active_flow_daily
```text
trade_date
symbol
active_buy_value
active_sell_value
active_buy_volume
active_sell_volume
unknown_value
unknown_volume
total_matched_value
total_matched_volume
flow_method
source_id
coverage_ratio
quality_score
quality_flags
created_at
updated_at
```

### ohlcv_daily
```text
trade_date
symbol
open
high
low
close
matched_volume
total_volume
matched_value
total_value
source_id
quality_flags
```

### bb_symbol_daily
```text
trade_date
symbol
horizon
bb_value
buy_measure
sell_measure
activity_measure
flow_method
source_id
method_version
coverage_ratio
quality_score
direction
turn_event
created_at
```

### bb_market_daily
```text
trade_date
universe_version
horizon
bb_value
aggregate_buy_value
aggregate_sell_value
flow_method
method_version
coverage_symbol_count
coverage_value_ratio
quality_score
created_at
```

### bb_breadth_daily
```text
trade_date
universe_version
horizon
positive_count
negative_count
neutral_count
positive_ratio
negative_ratio
neutral_ratio
threshold_version
flow_method
created_at
```

## Source priority config

```text
flow_source_priority:
  - DORAEMON_EXISTING_TRUE_FLOW
  - VNSTOCK_DATA_ACTIVE_FLOW
  - FIREANT_ACTIVE_FLOW
  - SSI_SELF_COLLECTED
  - OHLCV_PROXY
```

## Lineage

Mọi BB value phải trace được tới source, method, version, input range và quality.


---

# 06 — BB Engine Specification

## 1. Active Trade Flow Value

Daily:

```text
B_t = active_buy_value
S_t = active_sell_value
```

Horizon H:

```text
Buy_H  = rolling_sum(B_t, H)
Sell_H = rolling_sum(S_t, H)
BB_H   = 100 * Buy_H / (Buy_H + Sell_H)
```

Denominator=0 -> null.

## 2. Horizons

```text
3,5,10,20,50,200
```

UI default: 03/05/20/50/200; T10 optional.

## 3. Rolling semantics

T20 day t = `[t-19 ... t]`; ngày sau trượt 1 phiên. Không block reset. Không EMA mặc định.

## 4. Interpretation

- 50: active buy/sell balance.
- >50: buyer-initiated dominance.
- <50: seller-initiated dominance.

Không diễn giải là capital inflow kế toán.

## 5. Volume method

```text
BBV_H = 100 * Sum(BuyVolume) / Sum(BuyVolume + SellVolume)
```

Method=`ACTIVE_TRADE_FLOW_VOLUME`.

## 6. Unknown

Unknown loại khỏi numerator/denominator; coverage lưu riêng.

## 7. Activity

Giữ độc lập:

```text
activity_value_H
activity_ratio = TodayMatchedValue / AvgMatchedValue20
```

Không blend Activity vào BB.

## 8. Research events

Candidate causal TURN_UP(L):

```text
BB[t-2] >= BB[t-1]
BB[t] > BB[t-1]
BB[t-1] <= L
```

TURN_DOWN đối xứng.

20/30/70/80 là research references, chưa freeze.

## 9. Confluence

```text
spread = max(BB_h) - min(BB_h)
```

Nếu spread nhỏ và selected slopes cùng hướng -> candidate CONFLUENCE_UP/DOWN.
Tolerance phải empirical.

## 10. No look-ahead

Ngày t chỉ dùng data <= t; EOD signal chỉ available sau khi data cuối phiên finalized.


---

# 07 — Fallback & Method Boundaries

## OHLCV Proxy V1

```text
true_high = max(high_t, close_{t-1})
true_low  = min(low_t, close_{t-1})

p_t = (2*close_t - true_high - true_low)
      / (true_high - true_low)

spv_t = p_t * trading_value_t

proxy_flow_H = Sum(spv,H) / Sum(trading_value,H)
ProxyBB_H = 50 * (1 + proxy_flow_H)
```

Clamp pressure [-1,+1]. Zero range -> p=0.

## Trading value priority

1. actual matched value;
2. Typical Price × matched volume;
3. Typical Price × total volume.

## Label

```text
flow_method = OHLCV_PROXY
```

UI nên gọi `Technical Flow BB`, không gọi True Flow.

## Active Volume Value Estimate

Nếu có active volumes nhưng không active values:

```text
estimated_buy_value  = active_buy_volume  * representative_price
estimated_sell_value = active_sell_volume * representative_price
```

Method=`ACTIVE_VOLUME_VALUE_ESTIMATE`.

## Không dùng order intent như executed flow

`buy_orders/sell_orders/placed buy_volume/sell_volume` không được map vào Active Buy/Sell.


---

# 08 — SUMI-420 Market Specification

## Definition

“Toàn thị trường” trong Sumi V1 = `SUMI-420`.

Khoảng 420 mã trọng yếu, managed và version hóa.

## Market BB with value

Daily:

```text
MarketBuy_t  = Sum(active_buy_value_i)
MarketSell_t = Sum(active_sell_value_i)
```

Horizon:

```text
MarketBB_H = 100 * Sum_H(MarketBuy_t)
                  / Sum_H(MarketBuy_t + MarketSell_t)
```

Không average BB từng mã.

## Only active volume

Không cộng raw volumes trực tiếp qua cổ phiếu giá khác nhau.
Ước lượng value bằng representative price rồi aggregate.

## Breadth

```text
positive = count(BB_H > 50 + buffer)
negative = count(BB_H < 50 - buffer)
neutral  = remainder
```

Market BB = value dominance.
Breadth = participation.

## Coverage

```text
symbol_coverage
value_coverage
```

phải expose trong output.

## Universe version

Nếu thay mã, tạo version mới. Không cần reconstruct tất cả lịch sử thị trường Việt Nam.


---

# 10 — DEV Implementation Plan

## Phase 0 — Contracts ngay
- enums `flow_method`;
- DB migrations;
- adapter interfaces;
- SUMI-420 tables;
- calculator contracts;
- source/method/version metadata.

Không phụ thuộc vendor.

## Phase 1 — OHLCV Proxy research
Dùng history hiện có tính BB03/05/10/20/50/200 và làm test harness.

## Phase 2 — Active Flow fixtures
CSV synthetic:

```text
date,symbol,active_buy_value,active_sell_value
```

để test full True Flow pipeline.

## Phase 3 — Adapter theo audit
Có thể là:

```text
VnstockDataActiveFlowAdapter
SSIStreamingTradeAdapter
FireAntDailyActiveFlowAdapter
CsvImportAdapter
UiPathDropFolderAdapter
```

## Phase 4 — Forward collector
Nếu chỉ có realtime:
- deploy sớm;
- raw persistence;
- EOD aggregate;
- quality report;
- alerts.

Mỗi ngày không collect là mất một ngày future history.

## Phase 5 — Historical backfill
Khi tìm được source, import + reconcile + rerun BB. Core không đổi.

## Phase 6 — SUMI-420 Market BB
Aggregate active values/estimates, breadth, coverage.

## Phase 7 — Empirical calibration
Không optimize P&L.
Nghiên cứu distribution, extremes, turns, confluence, Proxy-vs-True overlap.

## Framework Definition of Done

1. Add vendor bằng adapter không sửa BB formula.
2. Import CSV active flow là BB chạy.
3. Proxy coexist nhưng không conflated.
4. SUMI-420 chạy trên fixtures.
5. Mọi output trace source/method/version/quality.
6. Raw collector reprocess idempotently.


---

# 11 — Validation & Acceptance

## Core math
- Buy=Sell -> BB=50.
- Sell=0, Buy>0 -> 100.
- Buy=0, Sell>0 -> 0.
- BB always [0,100].
- T05 day 6 uses days 2–6.

## Method boundaries
- Proxy không được label True Flow.
- Source switch không mất lineage.
- Historical Proxy và forward True Flow phân biệt được.

## Data quality
- Unknown không tự gán.
- Coverage được tính.
- Missing row != zero trading.
- Stream duplicate idempotent.
- EOD aggregate reconcile với totals trong tolerance.

## SUMI-420
- Aggregate measures before ratio.
- Không average symbol BB.
- Universe version lưu rõ.
- Coverage expose.

## Source acceptance
Một source chỉ được approve `ACTIVE_TRADE_FLOW_VALUE` khi:
- semantics documented;
- units verified;
- sample manually checked;
- coverage/missingness known;
- access stable;
- license/use risk reviewed.

Tên field alone không đủ.


---

# 12 — References — snapshot 2026-09-12

## SSI FastConnect
Official streaming X-TRADE exposes LastPrice, LastVol, TotalVal, TotalVol and Side=SD/BU/Unknown.

https://guide.ssi.com.vn/ssi-products/tieng-viet/fastconnect-data/du-lieu-streaming

Change log: Buy up/Sell down added to X-TRADE on 2023-11-07.

https://guide.ssi.com.vn/ssi-products/change-log

## FireAnt
Daily Active Buy/Sell `_AC` data documented for listed stocks.

https://help.fireant.vn/fireant-metakit/cac-loai-du-lieu/

Big Trade uses active buy/sell volume/value concepts.

https://help.fireant.vn/fireant-web/thu-vien-chi-so/ky-thuat/big-trade/

## Vnstock Data
`order_flow()` and `order_flow_history()` docs include active buy/sell volume/value fields.

https://vnstocks.com/docs/vnstock-data/cau-truc-du-lieu/insights

`vnstock_data` is Sponsor/extended, distinct from free community vnstock.

https://vnstocks.com/docs/vnstock-data/gioi-thieu-vnstock-data

License notes: software license does not automatically grant third-party source-data rights.

https://www.vnstocks.com/onboard/giay-phep-su-dung

## CafeF via Vnstock
Order stats are order-count/placed buy/sell volume statistics, not necessarily executed active flow.

https://www.vnstocks.com/docs/vnstock-data/du-lieu-giao-dich

## Note
Vendor endpoints, packages, price and license may change. Re-audit before production use.
