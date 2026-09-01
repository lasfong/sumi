# Batch 2 — Chart UX & Multi-Timeframe Analysis

## Outcome
Solve P1 and P2 user experience and missing feature issues on the Replay Chart, specifically Risk-Reward dynamic updates, Ichimoku cloud filling, and Multi-Timeframe Analysis (MTFA).

## Context and problem
Addresses the remaining high-priority findings from `docs/testing/TRADING_LAB_BUG_AND_UX_REPORT.md` (Issues #3, #4, and #7) that were not covered by Batch 1's logic/performance fixes.

## In scope
- **Risk-Reward Tool (Issue #4)**: Make the R:R ratio update in real-time while the user drags anchors, rather than only updating on click-release.
- **Multi-Timeframe Analysis (Issue #7)**: Add a UI switcher (e.g., 1H, 1D, 1W) to the Replay Workspace header, and plumb the selected timeframe through the frontend API to the backend Replay service.
- **Ichimoku Cloud Fill (Issue #3)**: Implement a custom Lightweight Charts plugin to correctly render the filled Kumo cloud (polygon fill between Span A and Span B).

## Out of scope
- Backend Replay logic changes (timeframe aggregation is already supported by the backend).
- Other chart indicators not related to Ichimoku.

## Invariants
- MTFA must not leak future data. The backend `ReplayService.get_candles(target_timeframe)` already correctly filters by the current replay boundary.
- Indicator caching must incorporate the active timeframe to avoid rendering 1D indicators on 1W candles.

## Current architecture
- `SumiPrimitiveDrawingProvider.ts` computes R:R only on creation.
- `ReplayWorkspace.tsx` hardcodes the session's base timeframe in the UI.
- `IndicatorRenderRegistry.ts` maps Ichimoku Span A and Span B as two separate line series without fill.

## Target design
- **Risk-Reward**: Update `updateDrag` in `SumiPrimitiveDrawingProvider.ts` to calculate `riskRewardRatio` dynamically.
- **MTFA**: Manage a `currentTimeframe` state in `ReplayWorkspaceController`, pass it down to `getSessionCandles` and indicator data fetchers. Update the UI to include a Timeframe Switcher.
- **Ichimoku**: Create `IchimokuCloudPlugin.ts` implementing `ICustomSeriesPaneView` to draw a filled polygon on the canvas.

## Milestones
1. Risk-Reward dynamic ratio and MTFA UI plumbed. (Testable via UI interaction and network calls).
2. Ichimoku Custom Plugin implemented and integrated. (Testable via visual validation of the chart).

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| PRO-07 (Ichimoku) | Custom plugin implemented | Visual check of Kumo |
| PRO-09 (MTFA) | Timeframe switcher in UI | Can switch to 1W and see aggregated candles |
| PRO-08 (Drawings) | R:R updates during drag | Real-time text changes |

## Verification commands
- `./scripts/verify-v2.sh`
- `./scripts/run-product-uat.sh`

## Progress log
- (Pending)

## Decision log
- (Pending)
