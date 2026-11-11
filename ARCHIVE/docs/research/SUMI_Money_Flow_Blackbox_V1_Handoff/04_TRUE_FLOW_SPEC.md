# TRUE_FLOW Algorithm Specification

Đây là specification chuẩn khi dữ liệu có buyer-initiated / seller-initiated executed value trực tiếp hoặc có thể reconstruct với chất lượng đã được kiểm chứng.

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
