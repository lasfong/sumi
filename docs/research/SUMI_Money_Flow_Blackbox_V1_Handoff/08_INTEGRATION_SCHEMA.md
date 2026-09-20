# Output Schema & Integration Contract

Contract đầu ra để session kiến trúc SUMI chính, Signal Engine hoặc UI sử dụng Blackbox mà không cần biết chi tiết implementation bên trong.

# 14. Output schema (integration contract)

## 14.1 Per-scope / per-horizon output

> methodology_version: string  
> calculation_date: date  
> scope_type: SYMBOL \| INDEX \| EXCHANGE \| UNIVERSE \| MARKET  
> scope_id: string  
> horizon: 3 \| 5 \| 10 \| 20 \| 50 \| 200  
>   
> flow_method: EXECUTED_ORDER_FLOW \| CLASSIFIED_ORDER_FLOW \|
> OHLCV_PROXY  
> value_source: EXECUTED_VALUE \| MATCHED_VALUE \|
> ESTIMATED_TP_X_VOLUME  
>   
> bb_score: float\|null \# 0..100  
> oib_raw: float\|null \# -1..+1 for true/classified flow; proxy raw for
> OHLCV proxy  
> buy_value: decimal\|null \# true/classified flow modes  
> sell_value: decimal\|null  
> rolling_total_value: decimal\|null  
>   
> classification_coverage: float\|null  
> unclassified_value: decimal\|null  
> warmup_complete: bool  
> data_quality: GOOD \| DEGRADED \| INSUFFICIENT \| MISSING  
>   
> direction: RISING \| FALLING \| FLAT  
> regime: POSITIVE \| NEUTRAL \| NEGATIVE  
> regime_run_length: int  
>   
> turn_up_20: bool  
> turn_up_30: bool  
> turn_down_70: bool  
> turn_down_80: bool  
> confluence_up: bool  
> confluence_down: bool

## 14.2 Aggregate extras

> eligible_symbol_count  
> valid_symbol_count  
> coverage_ratio  
> positive_count_breadth  
> neutral_count_breadth  
> negative_count_breadth  
> positive_value_breadth  
> neutral_value_breadth  
> negative_value_breadth  
> market_total_trading_value
