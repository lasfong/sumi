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
