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
