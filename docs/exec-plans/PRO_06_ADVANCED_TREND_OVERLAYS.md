# PRO-06 — Advanced Trend Overlays

Status: `PREPARED — USER AUTHORIZED`

## Outcome

Keltner Channels (`kc`), Parabolic SAR (`psar`), and SuperTrend (`supertrend`) become complete, released product capabilities: backend calculation authority, explicit semantic render definitions as price overlays, channel/marker/direction/trend-color semantics, editable/persistent instances, and browser-proven replay behavior across gaps, nulls, scales, and navigation.

## Context and problem

PRO-05 is independently approved and closed. The backend `IndicatorEngine` already implements `kc`, `psar`, and `supertrend`, but they were unreleased in the frontend. In the frontend, `IndicatorInstanceV1` currently defines 13 released indicators (EMA, RSI, MACD, CCI, raw Volume, SMA, BBands, ATR, Volume SMA, MFI, Stochastic, ADX, Relative Strength). Keltner Channels, PSAR, and SuperTrend must be added to the typed catalog with strict parameter-exact output mapping, multi-series all-or-nothing rendering, price overlay placement, direction/color semantics, and full UI/re-render lifecycle.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs PRO-IND-01..08 and PRO-IND-11; `docs/program/PRO_06_ADVANCED_TREND_OVERLAYS.md`; V3 G-01..05 regression.

## In scope

- Capture and fixture exact pinned backend column outputs for:
  - `kc`: `KCUe_${length}_${scalarStr}` (upper), `KCBe_${length}_${scalarStr}` (middle), `KCLe_${length}_${scalarStr}` (lower) as price overlay channels.
  - `psar`: `PSARl_${af0}_${max_af}` and `PSARs_${af0}_${max_af}` as price overlay stops/markers.
  - `supertrend`: `SUPERT_${length}_${multiplierStr}`, `SUPERTd_${length}_${multiplierStr}`, `SUPERTl_...`, `SUPERTs_...` as price overlay trend line with bull/bear direction semantics.
- Support parameter configuration for `kc` (`length`, `scalar`), `psar` (`af0`, `af`, `max_af`), and `supertrend` (`length`, `multiplier`) in `StrategyIndicatorAdapter`.
- Extend `frontend/src/features/indicators/indicatorDomain.ts` with typed definitions, parameters, default styles, and price overlay placements for `kc`, `psar`, and `supertrend`.
- Extend `frontend/src/components/chart/IndicatorRenderRegistry.ts` with strict parameter-exact column matching, deterministic float/scalar formatting, and multi-series all-or-nothing mapping for price overlays.
- Preserve full lifecycle: add, edit parameters, change color/styles, duplicate, hide, reorder, remove, reload, route navigation, and replay stepping.
- Add backend unit, edge, and parity tests in `test_indicators.py` and `test_indicator_parity_e2e.py`.
- Add frontend unit tests in `IndicatorRenderRegistry.test.ts` and `indicatorDomain.test.ts`.
- Extend deterministic Product UAT (`scripts/product-uat.mjs` and baseline manifest) with focused assertions and retained screenshots at 1440×1000 and 1280×800.

## Out of scope

- Ichimoku Cloud (PRO-07).
- Trade Planning and Journal (PRO-08).
- Strategy Research UX (PRO-09).
- New chart, drawing, or indicator dependencies.
- Changes to accepted formulas merely to match frontend expectations.
- Market-data provider/sync, packaging, commit, push, or release work.

## Invariants

- Backend `IndicatorEngine` remains calculation and parameter authority.
- Replay APIs and indicator responses expose no candle or value beyond `current_index`.
- No test or UAT mutates `backend/sumi.db`; record SHA-256 before/after gates.
- Indicator state remains explicit and serializable: identity, definition, parameters, pane, visibility, styles, and order.
- No assertion may be weakened or made non-blocking to obtain green results.

## Target design

### Indicator Catalog Additions

| ID | Label | Placement | Series Keys | Styles / Colors | Parameters |
| --- | --- | --- | --- | --- | --- |
| `kc` | Keltner Channels | price | `upper`, `middle`, `lower` | upper: `#00E5FF`, middle: `#FFD166`, lower: `#00E5FF` | `length` (20), `scalar` (2.0) |
| `psar` | Parabolic SAR | price | `sar` (or `long`/`short`) | sar: `#E040FB` | `af0` (0.02), `af` (0.02), `max_af` (0.2) |
| `supertrend` | SuperTrend | price | `supertrend` (or `long`/`short`) | bull: `#26A69A`, bear: `#EF5350` | `length` (7), `multiplier` (3.0) |

## Milestones

1. **Backend calculation & Adapter support:** Verify `kc`, `psar`, and `supertrend` outputs in `IndicatorEngine`; update `StrategyIndicatorAdapter._params_for` and `IndicatorConfig`; add unit/parity tests.
2. **Typed released catalog:** Extend `indicatorDomain.ts` types, schemas, default styles, and descriptors for `kc`, `psar`, and `supertrend`.
3. **Renderer adapters & tests:** Implement exact parameter-column adapters, all-or-nothing multi-series handling, and price overlay mapping in `IndicatorRenderRegistry.ts`; add focused vitest tests.
4. **Lifecycle & UI integration:** Verify manager dialog, price overlay chrome display, parameter edits, reload, and navigation.
5. **Product UAT & evidence:** Extend `product-uat.mjs` and baseline manifest with PRO-06 assertions; execute full technical gate and Product UAT; inspect screenshots; verify DB hash invariant; update records and stop at Reviewer gate.

## Acceptance mapping

| ID | Required implementation and evidence |
| --- | --- |
| PRO-IND-01 | Registry-driven parameters and backend calculations; parity fixtures for Keltner Channels, PSAR, SuperTrend. |
| PRO-IND-02 | Exhaustive released catalog with placement, series, format, scale, style, references, warm-up. |
| PRO-IND-03 | Tests proving unknown/engine-only IDs cannot render as EMA or appear released. |
| PRO-IND-04 | Semantic series keys/labels for Keltner Channels (`upper`/`middle`/`lower`), PSAR, SuperTrend. |
| PRO-IND-05 | Focused overlay, channel, and trend-color renderer contracts. |
| PRO-IND-06 | Multiple instances and complete manager/persistence/reload/resume lifecycle. |
| PRO-IND-07 | Null/warm-up/gap/replay-boundary fixtures without invalid segments or future data. |
| PRO-IND-08 | Backend parity covers every released output series and representative edges. |
| PRO-IND-11 | Retained browser evidence for labels, values, scales, panes, settings, persistence, and navigation. |

## Verification commands

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py -v
Set-Location ..\frontend
npm.cmd test -- --run src/features/indicators src/components/chart/__tests__/IndicatorRenderRegistry.test.ts src/components/chart/__tests__/SeriesManager.test.ts src/components/chart/__tests__/PaneManager.test.ts src/components/chart/__tests__/IndicatorPaneChrome.test.tsx
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

## Progress log

- 2026-08-15: User authorized PRO-06. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
