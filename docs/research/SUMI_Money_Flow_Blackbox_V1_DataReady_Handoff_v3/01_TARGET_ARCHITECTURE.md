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
