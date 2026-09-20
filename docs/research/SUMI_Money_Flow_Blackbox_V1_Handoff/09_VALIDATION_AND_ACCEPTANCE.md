# Validation, No-Look-Ahead & Acceptance Tests

Bộ yêu cầu để chứng minh implementation deterministic, không look-ahead, không survivorship bias và không đánh tráo semantics. Đây là tài liệu QA/Research Validation bắt buộc.

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
