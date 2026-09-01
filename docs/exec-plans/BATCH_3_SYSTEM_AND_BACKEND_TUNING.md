# BATCH 3 — System & Backend Tuning

## Outcome
The user can seamlessly draw risk-reward and trend lines into the future whitespace without the chart blocking the drag action. All user simulated trades strictly enforce the Vietnam stock market 100-share even-lot rule (lô chẵn), preventing fractional or odd-lot executions. 

## Context and problem
This batch addresses the remaining P1/P2 issues from the Trading Lab Audit:
- **Issue #2 (P1)**: The chart drawing tools (Risk-Reward, Trendline) cannot extend into the future because Lightweight Charts restricts `coordinateToTime` to existing data points.
- **Issue #8 (P1)**: The backend allows practice orders for non-round lots (e.g. 150 shares), violating standard Vietnam market rules.

## In scope
- Appending future whitespace/virtual bars to the chart series to enable proper `coordinateToTime` mapping into the future.
- Adding strict 100-share quantity validation to the backend order submission logic (`TradeLifecycleService.process_decision` and API schema).
- Rounding or validating the trading quantity input field in the frontend `TradeControls`.

## Out of scope
- Advanced drawing tools outside of Risk-Reward and Trendlines.
- Odd-lot trading (giao dịch lô lẻ). The system will strictly require multiples of 100.

## Invariants
- Adding future whitespace to the frontend MUST NOT leak future price or volume data from the backend. The backend must still only return data up to `current_index`.
- The local SQLite `backend/sumi.db` must not be mutated during automated test execution.

## Current architecture
- **Issue #2**: Frontend `SumiPrimitiveDrawingProvider.ts` relies on `coordinateToTime`. Lightweight charts uses the available dataset's `time` index. If the data ends at `current_index`, the chart's time scale has no knowledge of future dates, so `coordinateToTime` returns `null` for X coordinates to the right of the last bar.
- **Issue #8**: Backend `app/schemas/decision_schema.py` and `app/services/trade_lifecycle_service.py` currently accept any positive integer for order quantity.

## Target design
- **Future Whitespace**: We will compute and append `futureCount` (e.g., 100) empty whitespace objects `{ time: futureDate }` to the `candles` series in `SeriesManager.ts`. The dates will be calculated by skipping weekends (simple business day projection). This extends the internal timeframe, allowing Lightweight Charts to resolve X coordinates to future dates.
- **Lot Validation**:
  - **Backend**: Add a Pydantic validator to `DecisionCreate` or logic in `TradeLifecycleService.process_decision` to reject `quantity % 100 != 0` with a 400 error.
  - **Frontend**: Update `TradeControls.tsx` to automatically step by 100 and validate `quantity % 100 === 0`. Disable the BUY/SELL buttons if invalid.

## Milestones
1. Implement Backend Lot validation and Frontend Trade Controls input restrictions.
2. Implement Future Whitespace appending in `SeriesManager.ts` and verify drawing extension into the future.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| ACC-BATCH3-01 | `TradeLifecycleService` rejects quantity % 100 != 0 | Unit tests in `test_t2_settlement.py` |
| ACC-BATCH3-02 | `TradeControls.tsx` enforces step 100 | Manual UI verification |
| ACC-BATCH3-03 | `SeriesManager.ts` appends whitespace bars | `SumiPrimitiveDrawingProvider` can drag into future |

## Verification commands
```bash
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
```

## Rollback and compatibility
The drawing whitespace is purely a frontend presentation change. The 100-lot rule is backward compatible but prevents future odd-lot trades.

## Risks and mitigations
- **Data Leakage Risk**: Appending `{ time }` to `CandlestickSeries` might accidentally render as null bars. Using `WhitespaceData` (only `{ time }` property) natively supported by Lightweight Charts mitigates this.
- **Calendar Risk**: Projecting business days in the frontend might differ slightly from the backend's real calendar, but for chart geometry, approximate future business days (skipping Saturday/Sunday) are acceptable.

## Progress log
- 

## Decision log
- 

## Completion evidence
- 
