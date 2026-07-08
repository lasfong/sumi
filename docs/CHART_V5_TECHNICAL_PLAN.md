# Sumi Chart V5 - Technical Plan and Implementation Record

Date: 2026-07-03

## Decision

Sumi standardizes the replay chart on TradingView Lightweight Charts v5.2.
This is a selective rewrite of the chart module, not a rewrite of the product.
Backend session-scoped indicator responses remain the source of truth.

## Implemented boundaries

- `ChartWorkspace`: React lifecycle and chart facade.
- `PaneManager`: official v5 pane creation/removal and sizing.
- `SeriesManager`: candle, volume, indicator and marker series lifecycle.
- `IndicatorRenderRegistry`: backend column-to-visual mapping only; no formulas.
- `DrawingToolRegistry`: Sumi drawing rendering and preview lifecycle.
- `SumiDrawingAdapter`: validated drawing serialization/deserialization.
- `WorkspacePersistence`: versioned workspace serialization boundary.

Price pane contains candles, overlays and drawings. Volume always has its own
pane. Each oscillator type receives a dedicated official v5 pane, including
RSI, MACD and CCI. A single chart time scale keeps all panes synchronized when
replay moves forward or backward. Active indicator configurations are restored
per session from local workspace storage; drawings remain persisted by the
backend drawing endpoint.

## Community library spike

Repositories were cloned under `/tmp` for review and were not copied into the
Sumi source tree.

| Repository | Reviewed commit | License | Decision |
| --- | --- | --- | --- |
| deepentropy/lightweight-charts-indicators | `164e5ac` | MIT | Reference only. It calculates indicators in the browser and depends on `oakscriptjs`, conflicting with the backend-source-of-truth rule. |
| deepentropy/lightweight-charts-drawing | `5f2afc3` | MIT | Viable future provider behind `SumiDrawingAdapter`; current Sumi tools remain smaller and already match persisted data. |
| difurious/lightweight-charts-line-tools-core | `167a83c` | MPL-2.0 | Reference only. Requires v5.2, has no automated tests, and adds MPL file-level obligations. Reassess when advanced tools are prioritized. |
| tradingview/lightweight-charts | `868cae2` (`v5.2.0`) | Apache-2.0 | Production dependency. Official pane and series APIs are used directly. |

No community package is a production dependency in this change. Any future
provider must implement the Sumi adapter contract, preserve backend drawing
JSON compatibility, pass the acceptance suite and receive a license review.

## Risks and controls

- v4 to v5 API breakage: all legacy `add*Series` calls were migrated and the
  production TypeScript build is a required gate.
- Pane leaks: pane ownership is centralized and empty oscillator panes are
  removed with their final series.
- Future-data leakage: replay uses only the session indicator endpoint.
- Backend output naming: visual conventions are isolated in one registry and
  covered by tests, including MACD line/signal/histogram.
- Indicator dependency correctness: `pandas-ta==0.4.71b0` is pinned. Its CCI
  implementation has a precedence defect, so Sumi computes canonical CCI in
  `IndicatorEngine` and verifies it against the formula in a regression test.
- Demo/UAT data coverage: deterministic seed data now includes `FPT`, `SSI`,
  `VCI` and `VNINDEX`, matching the default Backtest, Strategy Lab and Scanner
  workflows.
- Default strategy UX: Backtest and Scanner derive the first available strategy
  as the default selection, so a new user can run the default workflow without
  first resolving an empty strategy select.
- Corrupt persisted drawings: deserialization validates every drawing and point.
- Community package maturity: no package enters production directly; adapters
  and a spike are mandatory.

## Acceptance checklist

- [x] Lightweight Charts is `^5.2.0`.
- [x] Volume is rendered in its own official pane.
- [x] RSI maps to its own pane.
- [x] MACD maps to its own pane with line, signal and histogram.
- [x] CCI maps to its own pane.
- [x] EMA maps to the price pane.
- [x] Drawings serialize and deserialize without changing their persisted form.
- [x] Active indicator workspace restores after page reload.
- [x] Replay indicator requests remain session-scoped.
- [x] Next/previous uses one chart and therefore one synchronized time scale.
- [x] Frontend production build, lint and unit tests pass.
- [x] Backend full suite passes in a clean Python 3.12 environment (48 passed,
  1 optional browser test skipped).
- [x] Browser acceptance passes against a deterministic seeded local stack.
  Verified official panes, EMA warm-up handling, CCI range, replay navigation,
  responsive layout and drawing persistence after reload with no console errors.
- [x] Backtest default workflow runs from the browser and renders metrics plus
  the equity chart.
- [x] Strategy Lab compares both bundled strategies over default symbols.
- [x] Signal Scanner runs with default symbols and benchmark, saves scan
  history and creates a replay session from a selected signal.
- [x] Analytics, Journal and Import pages render their expected empty/default
  states without console errors.

## Rollback

The work is isolated on `codex/lightweight-charts-v5-spike`. Rollback is a
branch-level operation; persisted drawing JSON is backward-compatible and no
database migration is required.
