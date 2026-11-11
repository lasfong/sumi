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
