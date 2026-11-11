# Architecture Decision Record — Money Flow Blackbox V1

Quyết định kiến trúc chính: ưu tiên TRUE_FLOW từ executed aggressive order flow; chỉ dùng OHLCV_PROXY khi dữ liệu không cho phép quan sát/reconstruct flow thật. File này là contract cấp kiến trúc.

# 3. Target architecture

> DATA SOURCES  
> \|  
> +-- Tier A: executed trades + aggressor_side ---------------------\>
> TRUE_FLOW  
> \|  
> +-- Tier B: executed trades + bid/ask quotes --\> classification --\>
> TRUE_FLOW_ESTIMATED  
> \|  
> +-- Tier C: tick trades only ------------------\> tick-test research
> -\> FLOW_ESTIMATED_LOW_CONFIDENCE  
> \|  
> +-- Tier D: daily OHLCV / trading value --------------------------\>
> OHLCV_PROXY  
>   
> ALL MODES  
> -\> daily primitive  
> -\> rolling T03/T05/T10/T20/T50/T200  
> -\> symbol BB  
> -\> aggregate market BB  
> -\> breadth / activity / quality metadata  
> -\> UI + downstream Signal Engine (read-only dependency)

The calculation layer must expose flow_method and quality flags with
every observation. A historical series may have different availability
by date, but DEV must not splice incompatible modes into one unlabeled
series.

# 10. Mode selection / graceful degradation

| **Condition**                                           | **Mode**            | **Required output label**         | **Allowed claim**                                                             |
|---------------------------------------------------------|---------------------|-----------------------------------|-------------------------------------------------------------------------------|
| Direct executed aggressor side available and documented | TRUE_FLOW           | flow_method=EXECUTED_ORDER_FLOW   | Buyer-/seller-initiated executed value dominance.                             |
| Trades + quotes; classifier validated                   | TRUE_FLOW_ESTIMATED | flow_method=CLASSIFIED_ORDER_FLOW | Estimated buyer-/seller-initiated dominance; publish coverage/error metadata. |
| Ticks only; classifier exploratory                      | RESEARCH_ONLY       | flow_method=TICK_TEST_ESTIMATE    | Research output only until validation threshold is met.                       |
| Daily OHLC + matched value/volume only                  | OHLCV_PROXY         | flow_method=OHLCV_PROXY           | Technical buying/selling pressure proxy only.                                 |

Never automatically switch a single displayed series from TRUE_FLOW to
OHLCV_PROXY without a visible method boundary. Prefer separate series
IDs or an explicit mode segment marker.

# 22. Final handoff decision

DEV should not start by coding an indicator. DEV should start by proving
what “flow” can be observed from the available public datasets. The
algorithm path is then deterministic:

> IF reliable executed aggressor-side data exists:  
> implement TRUE_FLOW BB  
> ELSE IF trades + quotes allow validated classification:  
> implement TRUE_FLOW_ESTIMATED BB  
> ELSE:  
> implement OHLCV_PROXY BB and label it explicitly  
>   
> In all cases:  
> rolling 03/05/10/20/50/200  
> point-in-time universe  
> market aggregate values first  
> breadth + activity + data-quality metadata  
> no look-ahead  
> no institutional/retail claims without classified data

The most important product invariant is semantic honesty: a technically
useful proxy is acceptable; mislabeling a proxy as real money flow is
not.
