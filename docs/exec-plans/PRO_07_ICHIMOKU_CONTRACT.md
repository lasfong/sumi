# PRO-07 — Ichimoku Contract

Status: `PREPARED — USER AUTHORIZED`

## Outcome

Ichimoku Cloud (`ichimoku`) is released as a complete, trustworthy product capability: backend calculation authority, strict no-look-ahead displacement contract, explicit multi-series render definitions as price overlays (Tenkan-sen, Kijun-sen, Senkou Span A, Senkou Span B, Chikou Span), editable/persistent instances, and browser-proven replay behavior across start/end boundaries, gaps, warm-up, replay stepping, rewind, and reload.

## Context and problem

PRO-06 is independently approved and closed. The backend `IndicatorEngine` implements `ichimoku`, but it remained unreleased in the frontend due to the complexity of displacement and future-information safety. In technical analysis, Ichimoku uses forward displacement for the Kumo cloud (Senkou Span A & B projected forward by `kijun` periods) and backward displacement for Chikou Span (plotted `kijun` periods in the past). In a replay workstation, forward values must never leak future candle information or fabricate unseen timestamps, while calculation timestamps and display timestamps must remain explicit, transparent, and replay-safe.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs PRO-IND-01..11 (especially PRO-IND-10); `docs/program/PRO_07_ICHIMOKU_CONTRACT.md`; V3 G-01..05 regression.

## In scope

- Capture and fixture exact pinned backend column outputs for:
  - `ichimoku`: `ITS_${tenkan}` (Tenkan-sen), `IKS_${kijun}` (Kijun-sen), `ISA_${tenkan}` (Senkou Span A), `ISB_${kijun}` (Senkou Span B), `ICS_${kijun}` (Chikou Span).
- Support parameter configuration for `ichimoku` (`tenkan`, `kijun`, `senkou`) in `StrategyIndicatorAdapter` and `IndicatorConfig`.
- Extend `frontend/src/features/indicators/indicatorDomain.ts` with typed definitions, parameters (`tenkan`: 9, `kijun`: 26, `senkou`: 52), default styles, and price overlay placement for `ichimoku`.
- Extend `frontend/src/components/chart/IndicatorRenderRegistry.ts` with strict parameter-exact column matching, multi-series all-or-nothing mapping, and semantic series keys:
  - `tenkan`: Conversion Line (`#26A69A`)
  - `kijun`: Base Line (`#EF5350`)
  - `spanA`: Senkou Span A (`#00E5FF`)
  - `spanB`: Senkou Span B (`#FF8A00`)
  - `chikou`: Chikou Span (`#E040FB`)
- Enforce the no-look-ahead invariant (PRO-IND-10): All series points are calculated strictly from candles up to `current_index`; forward cloud values display only valid projected values without leaking future price action.
- Preserve full lifecycle: add, edit parameters, change color/styles, duplicate, hide, reorder, remove, reload, route navigation, and replay stepping.
- Add backend unit, edge, parity, and displacement tests in `test_indicators.py` and `test_indicator_parity_e2e.py`.
- Add frontend unit tests in `IndicatorRenderRegistry.test.ts` and `indicatorDomain.test.ts`.
- Extend deterministic Product UAT (`scripts/product-uat.mjs` and baseline manifest) with focused assertions and retained screenshots at 1440×1000 and 1280×800.

## Out of scope

- Trade Planning and Journal (PRO-08).
- Strategy Research UX (PRO-09).
- New chart, drawing, or indicator dependencies.
- Changes to accepted formulas merely to match frontend expectations.
- Market-data provider/sync, packaging, commit, push, or release work.

## Invariants

- Backend `IndicatorEngine` remains calculation and parameter authority.
- Replay APIs and indicator responses expose no candle or value beyond `current_index`.
- Ichimoku displacement cannot reveal any future candle data or look-ahead signals.
- No test or UAT mutates `backend/sumi.db`; record SHA-256 before/after gates.
- Indicator state remains explicit and serializable: identity, definition, parameters, pane, visibility, styles, and order.
- No assertion may be weakened or made non-blocking to obtain green results.

## Target design

### Indicator Catalog Addition

| ID | Label | Placement | Series Keys | Styles / Colors | Parameters |
| --- | --- | --- | --- | --- | --- |
| `ichimoku` | Ichimoku Cloud | price | `tenkan`, `kijun`, `spanA`, `spanB`, `chikou` | tenkan: `#26A69A`, kijun: `#EF5350`, spanA: `#00E5FF`, spanB: `#FF8A00`, chikou: `#E040FB` | `tenkan` (9), `kijun` (26), `senkou` (52) |

## Milestones

1. **Backend calculation & Adapter support:** Verify `ichimoku` outputs in `IndicatorEngine`; update `StrategyIndicatorAdapter._params_for` and `IndicatorConfig`; add unit/parity/displacement tests in `test_indicators.py` and `test_indicator_parity_e2e.py`.
2. **Typed released catalog:** Extend `indicatorDomain.ts` types, schemas, default styles, and descriptors for `ichimoku`.
3. **Renderer adapters & tests:** Implement exact parameter-column adapters, all-or-nothing multi-series handling, and price overlay mapping for Ichimoku in `IndicatorRenderRegistry.ts`; add focused vitest tests in `IndicatorRenderRegistry.test.ts` and `indicatorDomain.test.ts`.
4. **Lifecycle & UI integration:** Update `IndicatorManager.tsx` empty-state copy; verify manager dialog, price overlay chrome display, parameter edits, reload, and navigation.
5. **Product UAT & evidence:** Extend `product-uat.mjs` and baseline manifest with PRO-07 assertions; execute full technical gate and Product UAT; inspect screenshots; verify DB hash invariant; update records and stop at Reviewer gate.

## Acceptance mapping

| ID | Required implementation and evidence |
| --- | --- |
| PRO-IND-01 | Registry-driven parameters and backend calculations; parity fixtures for Ichimoku. |
| PRO-IND-02 | Exhaustive released catalog with placement, series, format, scale, style, references, warm-up. |
| PRO-IND-03 | Tests proving unknown/engine-only IDs cannot render as EMA or appear released. |
| PRO-IND-04 | Semantic series keys/labels for Ichimoku (`tenkan`, `kijun`, `spanA`, `spanB`, `chikou`). |
| PRO-IND-05 | Focused overlay, multi-line, and cloud renderer contracts. |
| PRO-IND-06 | Multiple instances and complete manager/persistence/reload/resume lifecycle. |
| PRO-IND-07 | Null/warm-up/gap/replay-boundary fixtures without invalid segments or future data. |
| PRO-IND-08 | Backend parity covers every released output series and representative edges. |
| PRO-IND-10 | Ichimoku displacement is a documented display transform and cannot reveal a value calculated from a future candle. |
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

- 2026-08-16: User authorized PRO-07. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
