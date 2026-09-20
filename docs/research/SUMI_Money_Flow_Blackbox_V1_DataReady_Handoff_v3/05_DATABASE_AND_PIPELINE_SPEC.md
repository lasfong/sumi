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
