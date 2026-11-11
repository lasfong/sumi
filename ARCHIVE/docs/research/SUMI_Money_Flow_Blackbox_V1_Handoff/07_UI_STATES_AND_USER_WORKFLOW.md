# UI, States, Events & User Workflow

Tài liệu mô tả cách người dùng đọc BB03/05/20/50/200, các state/event nghiên cứu như turn-up, confluence, flow expansion/distribution và các default research parameters. Các threshold 20/30/70/80 là research defaults, chưa được tối ưu theo P&L.

# 11. UI semantics and user workflow

## 11.1 Default chart

- X-axis: trading date/time.

- Y-axis: BB value 0–100.

- Default lines: BB03, BB05, BB20, BB50, BB200; BB10 optional.

- Reference lines: 20, 30, 50, 70, 80 (configurable; only 50 is
  structurally fixed by the formula).

- Display current flow method and data-quality badge prominently.

## 11.2 How the user should read it

| **Pattern**                                                 | **Meaning in TRUE_FLOW mode**                                   | **Action semantics**                                |
|-------------------------------------------------------------|-----------------------------------------------------------------|-----------------------------------------------------|
| BB03 \< 20 then turns up                                    | Very short-horizon seller dominance is reversing toward buyers. | Watchlist / early flow reversal, not automatic BUY. |
| BB03 + BB05 turn up                                         | Short horizons agree.                                           | Stronger flow confirmation.                         |
| BB03 + BB05 + BB20 converge low then turn up                | Cross-horizon flow confluence from depressed buyer share.       | Candidate setup for Price/Volume confirmation.      |
| All fast lines \> 50 and rising                             | Buyer-initiated dominance across short/medium horizons.         | Positive flow regime.                               |
| BB03/05 turn down from high zone while BB50/200 remain high | Short-term outflow/profit-taking inside longer positive regime. | Risk/position-management context.                   |
| Multiple horizons cross below 50                            | Seller dominance broadens across time scales.                   | Distribution/outflow confirmation candidate.        |

BB is an analysis layer, not a standalone trading system. Signal Engine
may combine it later with Price and Volume rules, but this module must
stay deterministic and independently testable.

# 12. State and event research specification

The following are research candidates, not final trading thresholds. DEV
should implement events in a parameterized, no-look-ahead manner so they
can be evaluated on shape/distribution before being exposed as
production semantics.

| **Event**               | **Causal definition candidate**                                                                                           |
|-------------------------|---------------------------------------------------------------------------------------------------------------------------|
| TURN_UP(L)              | At t: BB\[t\] \> BB\[t-1\], BB\[t-1\] \<= L, and BB\[t-2\] \>= BB\[t-1\]. Uses only t-2..t.                               |
| TURN_DOWN(U)            | At t: BB\[t\] \< BB\[t-1\], BB\[t-1\] \>= U, and BB\[t-2\] \<= BB\[t-1\].                                                 |
| CONFLUENCE_UP(set, tol) | max(BB_h)-min(BB_h) \<= tol over selected horizons AND each selected slope \>= minimum slope.                             |
| CONFLUENCE_DOWN         | Symmetric negative version.                                                                                               |
| FLOW_EXPANSION_UP       | Fast horizons turn/cross upward sequentially and medium horizon slope becomes positive; exact rule to be research-tested. |
| REGIME_POSITIVE         | BB_H \> 50 + neutral_buffer for k consecutive sessions.                                                                   |
| REGIME_NEGATIVE         | BB_H \< 50 - neutral_buffer for k consecutive sessions.                                                                   |

20/30/70/80 are UI research defaults inspired by oscillator conventions,
not empirically optimized constants for SUMI. MFI documentation itself
notes that overbought/oversold levels can vary by market conditions and
are not standalone buy/sell reasons. \[R2\]

# 18. Default research parameters

> horizons = \[3, 5, 10, 20, 50, 200\]  
> default_visible_horizons = \[3, 5, 20, 50, 200\]  
> neutral_center = 50  
> reference_levels = \[20, 30, 50, 70, 80\]  
> allow_partial_horizon = false  
> market_aggregation = AGGREGATE_VALUES_THEN_RATIO  
> include_put_through = false \# default research policy; configurable
> after audit  
>   
> \# Event research only, not frozen trading parameters  
> turn_low_levels = \[20, 30\]  
> turn_high_levels = \[70, 80\]  
> neutral_buffer = configurable  
> confluence_tolerance = configurable  
> min_classification_coverage = research_to_validate

Parameters tied to data quality (classification coverage, quote
staleness, timestamp tolerance) belong to the research/admin
configuration, not ordinary end-user UI until validated.
