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
