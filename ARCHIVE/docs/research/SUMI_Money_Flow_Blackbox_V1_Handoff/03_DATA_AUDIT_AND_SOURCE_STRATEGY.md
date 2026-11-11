# Data Audit & Source Strategy

DEV phải hoàn thành Data Audit Gate **trước khi code core indicator**. Tài liệu này xác định các tier dữ liệu, checklist audit, cách phân loại trade khi không có aggressor side, và các câu hỏi cần đóng trước production freeze.

# 4. Data strategy and Data Audit Gate

## 4.1 Known public-data reality in Vietnam

Public sources can provide substantial Price/Volume/Trading-Value data,
foreign trading and some order statistics, but availability, historical
depth, field semantics and machine-readable access differ by
exchange/provider. HOSE publishes matched-trade versus put-through
trading statistics and buy/sell order statistics on its public data
pages; historical exchange reports also separate trading value and
foreign buy/sell value. A Vnstock integration document for CafeF exposes
daily fields such as matched_volume, foreign buy/sell volume,
proprietary buy/sell volume and order_stats (buy_orders, sell_orders,
buy_volume, sell_volume). These fields are useful, but order placement
statistics are not automatically executed aggressor-side flow.
\[R5\]\[R6\]\[R7\]

## 4.2 Data tiers

| **Tier**                              | **Minimum fields**                                                 | **What can be computed**                                     | **V1 recommendation**                                      |
|---------------------------------------|--------------------------------------------------------------------|--------------------------------------------------------------|------------------------------------------------------------|
| A - Direct aggressor                  | timestamp, symbol, trade_price, trade_volume/value, aggressor_side | Executed BuyValue/SellValue directly.                        | Best. Use TRUE_FLOW.                                       |
| B - Trade + quote                     | trade timestamp/price/size + synchronized best bid/ask             | Classify buy/sell with Lee-Ready-like method; estimated OIB. | Good. Use TRUE_FLOW_ESTIMATED with classification metrics. |
| C - Trade ticks only                  | timestamp, trade_price, trade_volume                               | Tick-test classification possible but lower confidence.      | Research only until validated against a labeled sample.    |
| D - Daily actual trading value + OHLC | date, O/H/L/C, matched volume/value                                | Continuous technical pressure weighted by actual value.      | Production fallback OHLCV_PROXY.                           |
| E - Daily OHLCV only                  | date, O/H/L/C, volume                                              | Proxy trading value + technical pressure.                    | Lowest production fallback; mark value_source=ESTIMATED.   |

## 4.3 Mandatory audit checklist

| **Audit question**                                    | **Pass condition**                                                               | **If failed**                                                  |
|-------------------------------------------------------|----------------------------------------------------------------------------------|----------------------------------------------------------------|
| Does source contain executed trades, not just orders? | Each row/event is a matched transaction or clearly defined execution aggregate.  | Do not call order stats executed flow.                         |
| Is aggressor side directly supplied?                  | Documented BUY/SELL side with clear semantics.                                   | Try Tier B classification.                                     |
| Can bid/ask be synchronized to trades?                | Timestamp precision and quote history adequate for deterministic classification. | Try Tier C or fallback.                                        |
| Can matched trading be separated from put-through?    | Fields or source docs identify matched vs negotiated/put-through.                | Default to exclude ambiguous put-through from core or flag it. |
| Historical depth by exchange?                         | Coverage matrix by exchange/date/symbol is known.                                | Mode may differ by period; never hide gaps.                    |
| Corporate actions / symbol lifecycle?                 | Point-in-time listing and status metadata available.                             | Build/obtain lifecycle table before whole-market backfill.     |
| Foreign/proprietary classification?                   | Documented buy/sell fields by group.                                             | Expose only where source exists; do not infer.                 |
| Order statistics semantics?                           | Clearly distinguishes placed, matched, cancelled, outstanding.                   | Treat as demand/supply telemetry, not TRUE_FLOW.               |

Deliverable from Data Audit: \`data_capability_matrix.csv\` with source
× exchange × date range × field × semantic confidence × missingness ×
update latency.

# 6. Trade-direction classification when aggressor side is absent

## 6.1 Preferred research method

Lee & Ready (1991) is the canonical reference: infer trade direction
using trade prices versus prevailing bid/ask quotes, with a tick test
for ambiguous/midpoint cases. The original paper also documents
synchronization and inside-spread problems; therefore this stage is
estimation, not ground truth. \[R1\]

> if trade_price \> quote_midpoint: BUY_INITIATED  
> elif trade_price \< quote_midpoint: SELL_INITIATED  
> else: apply tick rule using prior trade price direction  
>   
> \# exact quote-lag rule is NOT hard-coded from 1991 NYSE data;  
> \# it must be researched/validated for the timestamp behavior of each
> Vietnam feed.

Important: do not copy the historical “5-second quote lag” blindly. That
value addressed 1988 NYSE reporting behavior. DEV must validate time
synchronization for the specific Vietnamese source/feed.

## 6.2 Classification evaluation

- If any vendor/API provides a labeled aggressor-side sample, use it as
  validation ground truth.

- Report value-weighted and trade-count classification agreement, not
  just row accuracy.

- Report unclassified-at-midpoint rate and coverage by exchange/year.

- Run sensitivity tests for timestamp offsets and quote staleness.

- Freeze the classification algorithm/version; all BB outputs must
  record \`classification_method_version\`.

# 20. Open questions to close before production freeze

**1.** Which current CTCK/CafeF endpoints provide trade-level
executions, and how many years of history are available?

**2.** Does any source provide trustworthy aggressor side? If yes, what
is its exact definition and handling of ATO/ATC/put-through?

**3.** If classification must be reconstructed, what timestamp precision
and quote synchronization are available on HOSE/HNX/UPCoM?

**4.** What percentage of executed value can be classified reliably by
exchange/year?

**5.** Can matched trading value be obtained consistently for long
history, separately from put-through?

**6.** What is the best point-in-time universe source for listing,
delisting, suspension and security type?

**7.** What distribution do BB03/05/20/50/200 actually have on Vietnam
data, and are 20/80 useful extremes for every horizon?

**8.** How should ATO/ATC auctions be treated in TRUE_FLOW
classification if aggressor direction is not naturally defined?

**9.** When TRUE_FLOW exists only for recent years, should historical
OHLCV_PROXY be presented as a separate backfill panel rather than a
continuous series? (Recommended baseline: yes.)
