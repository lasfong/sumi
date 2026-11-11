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
