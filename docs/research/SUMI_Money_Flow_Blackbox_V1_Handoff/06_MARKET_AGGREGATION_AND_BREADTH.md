# Whole-Market Aggregation, Universe & Flow Breadth

Specification cho VNINDEX/index/exchange/universe/toàn thị trường. Nguyên tắc quan trọng: aggregate Buy/Sell/Flow Value trước rồi mới tính BB; không average BB score của từng mã.

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
