# 09 — Prompt chuyển cho project backend Doraemon

Copy nguyên prompt dưới đây.

---

# AUDIT DATA SOURCES FOR SUMI MONEY FLOW BLACKBOX

Bạn đang làm việc trong project backend **Doraemon**, hiện đã có nhiều source/API/collector cho chứng khoán Việt Nam.

Tôi đang xây module **SUMI Money Flow Blackbox (BB)**.

## Mục tiêu BB

Target tốt nhất là đo **Active Buy / Active Sell executed flow**:

```text
active_buy_value
active_sell_value
```

hoặc tối thiểu:

```text
active_buy_volume
active_sell_volume
```

Core formula nếu có value:

```text
BB_H = 100 * SUM(active_buy_value, H)
             / SUM(active_buy_value + active_sell_value, H)
```

Horizons:

```text
T03/T05/T10/T20/T50/T200
```

Market = universe khoảng **420 mã cố định/trọng yếu (SUMI-420)**.

## Nhiệm vụ

AUDIT TOÀN BỘ source, adapters, API endpoints, raw response samples, DB tables, collectors, caches và dependencies hiện có trong Doraemon để trả lời:

> Doraemon hiện có thể cung cấp data nào để triển khai Money Flow BB?

**Không đoán từ tên field. Phải kiểm tra semantics thực tế.**

## Data cần tìm — theo priority

### A. Active executed flow

Tìm mọi field tương đương:

```text
active_buy_value
active_sell_value
active_buy_volume
active_sell_volume
buy_up
sell_down
aggressor_side
trade_side
Side=BU/SD
net_active_buy_value
```

Kiểm tra history daily, current-session, raw tick.

### B. Raw trades

```text
trade timestamp
symbol
match price
match volume
trade value
side/aggressor
total volume/value
trading session
```

### C. Order book / quote

```text
best bid
best ask
bid volume
ask volume
timestamp
```

Mục đích: classifier nếu trade không có side.

### D. Daily matched trading

```text
matched_volume
matched_value
total_volume
total_value
put_through_volume
put_through_value
avg_match_price/VWAP
```

### E. Supplementary classified flows

```text
foreign buy/sell value
proprietary buy/sell value
institutional/retail nếu có classification thật
```

## Audit source nếu có

- KBS
- VCI/Vietcap
- CafeF
- FireAnt
- SSI/FastConnect
- VNDIRECT
- TCBS legacy
- vnstock/vnstock_data
- bất kỳ insight/order-flow endpoint khác

## Semantics warning

Không đánh đồng:

```text
buy_volume/sell_volume của ORDER STATS
```

với:

```text
ACTIVE BUY/ACTIVE SELL đã khớp
```

Order intent phải label `ORDER_INTENT`.
Bid/ask pending cũng không phải executed flow.

## Source matrix bắt buộc

| Source | API/Field | Semantics thực | Realtime | Historical | Start date | 420 coverage | Volume/Value | Auth | Rate limit | License/risk | Quality |

## Nếu source có Active Buy/Sell

Test ít nhất:
- ACB
- FPT
- HPG
- VCB
- 1 mã HNX
- 1 mã UPCOM

Test:
- ngày gần nhất;
- 1 tháng trước;
- 1 năm trước nếu endpoint cho phép.

Báo:
- rows;
- missing days;
- Unknown;
- units;
- response sample;
- endpoint/method/code location.

## Kiểm tra đặc biệt vnstock_data

Docs hiện mô tả:

```text
Insights().equity('ACB').order_flow()
```

và:

```text
Insights().equity('ACB').order_flow_history()
```

với active buy/sell value + volume.

Nếu môi trường có thể test:
- upstream source;
- historical coverage;
- tier/license;
- 420 symbols;
- batchability;
- persistence rights.

## Kiểm tra SSI FastConnect nếu có credential hợp lệ

Official X-TRADE có:

```text
LastPrice
LastVol
TotalVal
TotalVol
Side=BU/SD/Unknown
```

Nếu có key:
- test stream;
- subscribe 420/ALL;
- reconnect behavior;
- raw sample;
- historical endpoint hay forward only.

Không expose secrets.

## Output bắt buộc

```text
BB_DATA_AUDIT_REPORT.md
BB_DATA_SOURCE_MATRIX.md hoặc .csv
BB_SAMPLE_RESPONSES_REDACTED.md
BB_GAP_ANALYSIS.md
BB_RECOMMENDATION.md
```

`BB_RECOMMENDATION.md` phải kết luận một hoặc nhiều trạng thái:

```text
A. Historical ACTIVE_TRADE_FLOW_VALUE available
B. Historical ACTIVE_TRADE_FLOW_VOLUME available
C. Realtime True Flow only — start self-collection
D. Trade+Quote available — classifier research possible
E. Only OHLCV Proxy available
```

## Ranking source

1. semantics đúng;
2. history coverage;
3. reliability;
4. SUMI-420 coverage;
5. license clarity;
6. cost;
7. implementation complexity.

## Không làm

- Không production-code BB trong audit này.
- Không tự đổi architecture.
- Không optimize trading signals/P&L.
- Không gọi large order = institution nếu không có classification.
- Không ghi secrets vào output.

## 8 câu bắt buộc trả lời cuối report

1. Doraemon hiện có True Flow chưa?
2. Volume hay Value?
3. Historical bắt đầu khi nào?
4. Bao phủ SUMI-420 không?
5. Nếu chưa có history, realtime source nào phù hợp để tự collect?
6. Cần thêm source ngoài nào?
7. Có source free/đã có account dùng ngay không?
8. Có cần RPA không hay API/export đã đủ?
