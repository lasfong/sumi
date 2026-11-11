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
