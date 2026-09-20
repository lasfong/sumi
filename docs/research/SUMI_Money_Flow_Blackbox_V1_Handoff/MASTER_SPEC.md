**SUMI**

Money Flow Blackbox V1

Research Summary, Data Strategy, Algorithm Specification & DEV
Validation Plan

Status: DESIGN BASELINE FOR RESEARCH IMPLEMENTATION

Scope: Money Flow Blackbox only. This document does not define the full
Signal Engine or Backtest Engine.

Core principle: measure true executed buy/sell pressure when data
allows; otherwise run an explicitly labeled OHLCV technical proxy. Never
present the proxy as actual capital inflow/outflow.

**Prepared from the SUMI research/design session**  
**Baseline date:** 12 Sep 2026  
Version: 1.0-research

# 0. Executive decision

The research converged on a two-mode architecture. The preferred mode is
TRUE_FLOW, based on buyer-initiated versus seller-initiated executed
trading value. The fallback mode is OHLCV_PROXY, used only when public
data cannot reconstruct executed trade direction. These modes must
remain semantically and technically distinct.

| **Decision**        | **V1 baseline**                                                                                                          |
|---------------------|--------------------------------------------------------------------------------------------------------------------------|
| Preferred primitive | Executed aggressive Buy Value and Sell Value (buyer-/seller-initiated trades).                                           |
| Core oscillator     | BB_H = 100 × Buy_H / (Buy_H + Sell_H), equivalent to 50 × (1 + normalized order imbalance).                              |
| Horizons            | Rolling T03, T05, T10, T20, T50, T200. Default chart may show 03/05/20/50/200.                                           |
| Whole-market BB     | Aggregate Buy/Sell value across point-in-time universe first, then compute the ratio. Do not average symbol BB scores.   |
| Breadth             | Count breadth + trading-value breadth, separately from Market BB.                                                        |
| Fallback            | Continuous OHLCV technical flow proxy (True-Range/Chaikin-family pressure × Trading Value), clearly labeled OHLCV_PROXY. |
| Not allowed         | Claiming actual capital inflow/outflow, institutional/retail flow, or true aggressive buy/sell from daily OHLCV.         |
| Thresholds          | 50 is structurally meaningful. 20/30/70/80 remain research defaults, not optimized trading thresholds.                   |

GO/NO-GO rule: DEV must complete the Data Audit Gate before deciding
which mode can be implemented for each exchange, symbol history, and
date range.

# 1. Problem definition and terminology

## 1.1 What the Blackbox is intended to answer

- For a symbol: is aggressive trading pressure dominated by buyers or
  sellers, at T03/T05/T10/T20/T50/T200?

- For VNINDEX/index/universe/exchange/market: is aggregate trading
  pressure turning positive or negative, and how broad is participation?

- Can multiple horizons converge and turn together, creating a
  flow-confluence event worth further Price/Volume confirmation?

- Can the user distinguish short-horizon reversal from long-horizon
  regime without resetting every N sessions?

## 1.2 Mandatory terminology

| **Term**                       | **Definition**                                                                                                                                | **Can daily OHLCV prove it?**                                                        |
|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------|
| Trading Value / Turnover       | Gross value traded. Every executed trade has both a buyer and seller.                                                                         | Yes (exact if actual matched value exists; approximate from price×volume otherwise). |
| Actual capital inflow/outflow  | Cash entering/leaving an investment vehicle/account/group or primary issuance/redemption.                                                     | No.                                                                                  |
| Aggressive / initiated flow    | Executed value attributed to the side that initiated the trade (buyer taking the ask / seller hitting the bid, or equivalent classification). | No; needs transaction/quote or a trustworthy aggressor-side feed.                    |
| Order Flow Imbalance (OIB)     | Normalized difference between buyer-initiated and seller-initiated activity.                                                                  | No, unless trade direction can be reconstructed.                                     |
| Technical money-flow proxy     | Price/volume-based estimate of buying vs selling pressure.                                                                                    | Yes.                                                                                 |
| Foreign / proprietary net flow | Buy minus sell for a classified investor group.                                                                                               | Only if a source publishes the classification.                                       |
| Institutional / retail flow    | Buy/sell attributable to investor class.                                                                                                      | No, unless investor identity/classification is provided.                             |

Design rule: the UI/API must never silently collapse these concepts into
one generic “money in/out” field.

# 2. Why the research moved away from MFI/CMF as the primary core

Early design iterations evaluated MFI, CMF, ADL, OBV, Trading Value,
signed value, True-Range pressure, Force Index, VPT/PVT, Klinger and
related volume indicators. The important result is not that these
indicators are useless; it is that they remain price-volume inference
mechanisms. If executed trade direction is available, a direct
order-flow primitive is closer to the stated goal.

| **Candidate**               | **Useful information**                                                                           | **Why not TRUE_FLOW core**                                                                        |
|-----------------------------|--------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| MFI                         | 0–100 volume-weighted momentum; positive/negative “money flow” based on Typical Price direction. | Daily classification is price-derived and effectively binary; it does not observe aggressor side. |
| CMF                         | Continuous accumulation/distribution based on Close location within H–L, weighted by volume.     | Still infers pressure from candle geometry; does not observe buyer-/seller-initiated trades.      |
| Twiggs/True-Range flow      | Improves gap handling and smoothing over classic CMF.                                            | Still a technical proxy, not executed order flow.                                                 |
| ADL                         | Cumulative Chaikin accumulation/distribution primitive.                                          | Redundant with the CMF family and cumulative/unbounded.                                           |
| OBV                         | Simple sign of volume by close-to-close direction.                                               | Binary price-derived sign; cumulative and unbounded.                                              |
| Force Index / VPT / Klinger | Blend return magnitude, volume and smoothing.                                                    | More transformation/parameters without solving the core data-observation problem.                 |

Decision: keep standard indicators as optional confirmation/benchmark
tools, not as the definition of TRUE_FLOW BB.

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

# 5. TRUE_FLOW BB specification

## 5.1 Daily primitive

For each symbol i and session t, aggregate executed trades classified as
buyer-initiated or seller-initiated:

> buy_value\[i,t\] = Σ (trade_price × trade_volume) for buyer-initiated
> trades  
> sell_value\[i,t\] = Σ (trade_price × trade_volume) for
> seller-initiated trades  
> total_initiated_value\[i,t\] = buy_value + sell_value

If the feed directly provides executed buy/sell value, no trade-level
reconstruction is needed. If it provides only volumes, compute values at
trade level where possible; do not multiply an aggregate buy volume by
Close and call it exact executed value.

## 5.2 Normalized order imbalance and BB scale

> OIB_H = (BUY_H - SELL_H) / (BUY_H + SELL_H) \# \[-1, +1\]  
> BB_H = 50 × (1 + OIB_H) \# \[0, 100\]  
>   
> Equivalent:  
> BB_H = 100 × BUY_H / (BUY_H + SELL_H)

Interpretation: 50 is balanced aggressive executed value; above 50
indicates buyer-initiated dominance; below 50 indicates seller-initiated
dominance. This is an aggressive-order-flow measure, not “cash retained
inside the stock.”

## 5.3 Multi-horizon definition

V1 uses rolling sums, not block resets and not EMA smoothing:

> for H in \[3,5,10,20,50,200\]:  
> BUY_H\[t\] = sum(buy_value\[t-H+1 : t\])  
> SELL_H\[t\] = sum(sell_value\[t-H+1 : t\])  
> BB_H\[t\] = 100 \* BUY_H\[t\] / (BUY_H\[t\] + SELL_H\[t\])

A horizon is a time-scale of measured flow, not an investor identity.
T03 is not “retail”; T200 is not “institutional”.

## 5.4 Warm-up and zero-activity rules

- \`warmup_complete=false\` until H valid market sessions exist for a
  symbol. Default: no partial horizon score.

- If BUY_H + SELL_H = 0, BB_H = null (not 50). There was no classifiable
  initiated activity.

- If some trades are unclassified, keep \`classified_value\`,
  \`unclassified_value\`, and \`classification_coverage\` separately.

- Production threshold suggestion: do not publish TRUE_FLOW BB if
  classification coverage is below a configurable minimum; default
  research candidate = 90%, to be audited empirically rather than
  assumed.

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

# 7. Whole-market, exchange, universe and index scopes

## 7.1 Aggregate money first; never average symbol scores

> market_buy_value\[t\] = Σ_i buy_value\[i,t\]  
> market_sell_value\[t\] = Σ_i sell_value\[i,t\]  
>   
> MarketBB_H\[t\] = 100 × Σ_H market_buy_value / Σ_H (market_buy_value +
> market_sell_value)

This is trading-value weighting by construction. Equal-weight and
market-cap-weight do not answer the same question and are not the core
Market Flow metric.

## 7.2 Point-in-time universe

- Universe U_t must be reconstructed at each historical date.

- Include listed symbols while eligible; retain delisted symbols in
  historical calculations until their valid end date.

- Do not backfill today’s index constituents into past dates.

- Security-type filters must be explicit (equity, ETF, covered warrant,
  etc.). Default V1 equity market scope should be documented per
  deployment.

## 7.3 VNINDEX treatment

Price-chart behavior of VNINDEX can be treated as an index series, but
Money Flow BB should be computed from the eligible constituents/traded
equity universe rather than interpreting an index “volume” field as if
VNINDEX were a single security.

# 8. Flow Breadth and Activity

Market BB answers where trading value is concentrated. Breadth answers
how widely the direction is shared. Both are required.

> positive_count_breadth_H = count(BB_i,H \> 50 + buffer) /
> valid_count  
> negative_count_breadth_H = count(BB_i,H \< 50 - buffer) /
> valid_count  
>   
> positive_value_breadth_H = Σ rolling_total_value_i,H where BB_i,H \>
> 50+buffer / Σ rolling_total_value_i,H  
> negative_value_breadth_H = analogous

Default buffer is a research parameter; start with 0 for diagnostic
output, and optionally test a neutral zone such as 47.5–52.5. Do not
optimize the buffer on P&L.

Activity remains independent: publish total/rolling trading value and
relative activity (e.g., value / rolling median or EMA) alongside BB; do
not blend it into BB unless future research establishes a clear
requirement.

# 9. OHLCV_PROXY fallback specification

Use this mode only when executed buy/sell flow cannot be obtained or
reliably reconstructed. It is a technical pressure proxy, not actual
order flow.

## 9.1 Proxy primitive

> true_high\[t\] = max(high\[t\], close\[t-1\])  
> true_low\[t\] = min(low\[t\], close\[t-1\])  
>   
> pressure\[t\] = (2\*close\[t\] - true_high\[t\] - true_low\[t\]) /
> (true_high\[t\] - true_low\[t\])  
> \# pressure in \[-1, +1\]; if denominator=0 -\> 0  
>   
> trading_value\[t\] = actual_matched_value\[t\] if available  
> else typical_price\[t\] \* volume\[t\]  
>   
> signed_pressure_value\[t\] = pressure\[t\] \* trading_value\[t\]

## 9.2 Proxy BB horizon

> proxy_flow_H = Σ_H signed_pressure_value / Σ_H trading_value  
> ProxyBB_H = 50 × (1 + proxy_flow_H)

Why this proxy: it is continuous (unlike binary MFI/OBV direction),
incorporates gap information through previous Close, remains bounded, is
value-weighted, and aggregates cleanly from symbol to market. It is
still price-derived, therefore its API/UI label must say \`Technical
Flow Proxy\`.

## 9.3 Proxy data priority

| **Priority** | **Value source**               | **Notes**                                                                                                     |
|--------------|--------------------------------|---------------------------------------------------------------------------------------------------------------|
| 1            | Actual matched trading value   | Preferred. Exclude or separately flag put-through/negotiated trades unless research explicitly includes them. |
| 2            | Typical Price × matched volume | Fallback estimate when value is absent.                                                                       |
| 3            | Typical Price × total volume   | Last resort if matched vs negotiated volume cannot be separated; lower quality flag.                          |

## 9.4 MFI/CMF roles in proxy research

- MFI is a useful benchmark/confirmation oscillator and naturally uses
  Typical Price × Volume, but is not the proxy core because its daily
  positive/negative classification is price-derived and binary.
  \[R2\]\[R3\]

- CMF is a useful benchmark for accumulation/distribution logic. The
  proxy pressure is conceptually in the Chaikin family but uses a
  True-Range-style daily range to account for gaps. \[R4\]

- OBV and ADL are not required in BB V1. They may remain available
  elsewhere in the technical indicator library.

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

# 13. No-look-ahead and historical integrity

1.  All values at session t may use only records timestamped/known no
    later than the defined calculation cutoff for t.

2.  Rolling T20 at t is exactly t-19...t valid market sessions for that
    scope, not 20 arbitrary non-null rows.

3.  Turn events use only current and prior observations; no future pivot
    confirmation.

4.  Point-in-time universe and index membership must be used; no
    current-survivor universe backfill.

5.  Delisted securities remain in historical market calculations while
    they were eligible.

6.  Missing source data is not zero flow.

7.  Corporate actions must not silently fabricate proxy signals; price
    series policy must be explicit for OHLCV_PROXY.

8.  If TRUE_FLOW classification logic changes, recompute under a new
    methodology_version; do not overwrite historical semantics silently.

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

# 15. DEV implementation plan

| **Phase**                         | **Goal**                                                                                    | **Required deliverable**                           | **Exit criterion**                                                          |
|-----------------------------------|---------------------------------------------------------------------------------------------|----------------------------------------------------|-----------------------------------------------------------------------------|
| R0 - Source inventory             | Inventory every existing CTCK/CafeF/public endpoint and stored table.                       | source_catalog.md + sample payloads.               | All candidate fields have source/semantic/date coverage.                    |
| R1 - Capability audit             | Determine true-flow feasibility by source/exchange/history.                                 | data_capability_matrix.csv + DQ report.            | Mode decision possible for each data segment.                               |
| R2 - TRUE_FLOW research prototype | Implement deterministic trade classification and daily Buy/Sell aggregation where possible. | Research notebook/module + metrics.                | Classification coverage/accuracy meets documented threshold or is rejected. |
| R3 - OHLCV_PROXY prototype        | Implement fallback with actual matched value if available.                                  | Proxy research module + benchmark against MFI/CMF. | Deterministic output, bounded, no look-ahead.                               |
| R4 - Multi-horizon + market       | Add rolling horizons, point-in-time universe, market aggregation and breadth.               | Daily symbol + market datasets.                    | Identity/coverage tests pass.                                               |
| R5 - Visual evaluation            | Plot BB03/05/20/50/200 and annotate candidate events.                                       | Research dashboard/charts.                         | Behavior is interpretable; thresholds documented statistically.             |
| R6 - Freeze V1 spec               | Freeze methodology, parameters, schema and labels.                                          | versioned specification + golden fixtures.         | All acceptance tests pass; data claims match mode.                          |

# 16. Research evaluation plan (do not optimize on P&L)

The first evaluation is about measurement quality and behavior, not
profitability. Avoid selecting thresholds because they maximize future
returns.

- Distribution study by horizon, exchange, liquidity bucket and market
  regime: median, IQR, percentiles, time spent below 20/30 and above
  70/80.

- Smoothness/responsiveness: daily delta distribution, autocorrelation,
  frequency of 0/100 saturation, line crossing rate.

- Cross-horizon behavior: convergence frequency, lead/lag from
  BB03→BB05→BB20, persistence after turning.

- TRUE_FLOW vs OHLCV_PROXY overlap on periods where both exist:
  correlation, sign agreement around 50, event agreement, systematic
  biases.

- Source robustness: compare broker/public endpoints on same dates,
  detect revision or missing-session differences.

- Market aggregation diagnostics: concentration contribution by top
  symbols, breadth divergence cases, exchange decomposition.

- Only after measurement semantics are frozen should a separate backtest
  session study whether these events add predictive value.

# 17. Acceptance tests

| **ID**                       | **Acceptance criterion**                                                                                                          |
|------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| AT01 Bounds                  | Every valid BB is within \[0,100\]; every OIB/raw flow within \[-1,+1\].                                                          |
| AT02 Balance                 | If rolling BuyValue == SellValue \> 0 then BB == 50.                                                                              |
| AT03 All buys                | If SellValue==0 and BuyValue\>0 across the valid horizon then TRUE_FLOW BB==100.                                                  |
| AT04 All sells               | If BuyValue==0 and SellValue\>0 then TRUE_FLOW BB==0.                                                                             |
| AT05 Zero activity           | If Buy+Sell==0 then BB is null, never silently 50.                                                                                |
| AT06 Rolling not block       | T05 at day 6 uses sessions 2..6; no reset after session 5.                                                                        |
| AT07 Future invariance       | Appending future records cannot change any prior BB/state/event.                                                                  |
| AT08 Warm-up                 | T200 is null until 200 valid sessions under the chosen policy.                                                                    |
| AT09 Aggregate identity      | Market BB from aggregated Buy/Sell equals the ratio calculated from the same aggregated values.                                   |
| AT10 No score averaging      | A deliberately constructed dataset demonstrates that mean(symbol BB) != core Market BB; implementation must use aggregate values. |
| AT11 Universe PIT            | A delisted symbol contributes before delisting and not after; a newly listed symbol appears only from eligibility date.           |
| AT12 Missing != zero         | Missing vendor record triggers DQ/missing state, not zero buy/sell.                                                               |
| AT13 Method boundary         | Changing flow_method requires a visible method field and methodology version; no hidden splice.                                   |
| AT14 Classification coverage | TRUE_FLOW_ESTIMATED publishes classified/unclassified value and coverage; insufficient coverage blocks production score.          |
| AT15 Proxy formula           | OHLCV proxy: true-range pressure and score stay bounded; H==L/true range zero handled deterministically.                          |
| AT16 Value source            | If exact matched value is supplied, proxy output records MATCHED_VALUE; otherwise ESTIMATED_TP_X_VOLUME.                          |
| AT17 Breadth consistency     | Positive + neutral + negative count breadth sums to 1 within tolerance; value breadth likewise.                                   |
| AT18 Turn causality          | TURN_UP/DOWN at t is unchanged when t+1...future is modified.                                                                     |
| AT19 Put-through policy      | Matched vs put-through inclusion follows configured policy and is surfaced in metadata.                                           |
| AT20 Reproducibility         | Same input snapshot + methodology_version produces identical output bit-for-bit/within declared numeric tolerance.                |

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

# 19. Explicit non-goals / prohibited claims

- Do not call Trading Value “net money inflow”.

- Do not call OHLCV_PROXY “actual money in/out”.

- Do not infer institutional vs retail from T03/T200, trade size, candle
  shape, volume spike or MFI/CMF.

- Do not equate buy/sell order placement statistics with executed
  aggressor-side value unless source semantics prove this.

- Do not tune thresholds on backtest P&L during measurement-design
  research.

- Do not create a composite by averaging MFI + CMF + OBV + ADL; that
  would double-count overlapping price-volume information and obscure
  interpretation.

- Do not use current constituents/survivors for historical market
  breadth.

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

# 21. Source notes / research references

**\[R1\]** Lee, C.M.C. & Ready, M.J. (1991), “Inferring Trade Direction
from Intraday Data”, The Journal of Finance 46(2), 733–746.
[Source](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1991.tb02683.x)

**\[R2\]** Fidelity Technical Indicator Guide — Money Flow Index (MFI):
Typical Price × Volume; positive/negative money flow; 0–100 oscillator;
20/80 are conventional, not standalone trade rules.
[Source](https://www.fidelity.com/learning-center/trading-investing/technical-analysis/technical-indicator-guide/MFI)

**\[R3\]** Fidelity Technical Indicator Guide — Money Flow: Typical
Price = (H+L+C)/3; Money Flow = Typical Price × Volume.
[Source](https://www.fidelity.com/learning-center/trading-investing/technical-analysis/technical-indicator-guide/money-flow)

**\[R4\]** Fidelity Technical Indicator Guide — Chaikin Money Flow:
accumulation/distribution based on Close location within H–L,
volume-weighted rolling calculation.
[Source](https://www.fidelity.com/learning-center/trading-investing/technical-analysis/technical-indicator-guide/cmf)

**\[R5\]** HOSE public “Quy mô giao dịch” pages: separates matched
trading and put-through trading and exposes buy/sell-order statistics at
public interface level.
[Source](https://www.hsx.vn/vi/du-lieu-giao-dich/quy-mo-giao-dich)

**\[R6\]** HOSE historical annual reports: examples of matched vs
put-through Trading Value and foreign buy/sell Trading Value reporting.
[Source](https://staticfile.hsx.vn/Uploads/UploadDocuments/2372186/BAO%20CAO%20THUONG%20NIEN%202019_%20FINAL.pdf)

**\[R7\]** Vnstock data documentation — CafeF integration fields
including price history, foreign/proprietary trading and order
statistics. Treat as integration documentation, not proof that order
stats equal executed aggressor flow.
[Source](https://www.vnstocks.com/docs/vnstock-data/du-lieu-giao-dich)

**\[R8\]** Chordia, Roll & Subrahmanyam-related literature summarized in
“Order imbalance, liquidity, and market returns”: marketwide order
imbalance constructed by assigning transactions to
buyer-/seller-initiated categories.
[Source](https://www.sciencedirect.com/science/article/pii/S0304405X02001368)

**\[R9\]** Example recent academic use of dollar order imbalance:
aggregate buyer-initiated minus seller-initiated dollar volume scaled by
their sum.
[Source](https://onlinelibrary.wiley.com/doi/10.1111/1467-8551.12755)

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
