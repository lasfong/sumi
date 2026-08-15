# PRO-05 — Momentum and Relative Strength

Status: `PREPARED — USER AUTHORIZED`

## Outcome

MFI (Money Flow Index), Stochastic Oscillator, ADX (Average Directional Index), and Relative Strength versus VNINDEX (RS-VNINDEX) become complete, released product capabilities: backend calculation authority, benchmark date alignment without future data, explicit semantic render definitions, oscillator/subpane placement, editable/persistent instances, and browser-proven replay behavior.

## Context and problem

PRO-04 is independently approved and closed. The backend `IndicatorEngine` already implements `mfi`, `stoch`, and `adx`, but they are unreleased in the frontend. `relative_strength` (RS vs VNINDEX benchmark) needs an explicit backend implementation with date-aligned series matching and handling of missing benchmark dates or symbols. In the frontend, `IndicatorInstanceV1` currently defines 9 released indicators (EMA, RSI, MACD, CCI, raw Volume, SMA, BBands, ATR, Volume SMA). MFI, Stochastic, ADX, and Relative Strength must be added to the typed catalog with strict parameter-exact output mapping, multi-series all-or-nothing rendering, reference lines, and benchmark date alignment.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs PRO-IND-01..09 and PRO-IND-11; `docs/program/PRO_05_MOMENTUM_AND_RELATIVE_STRENGTH.md`; V3 G-01..05 regression.

## In scope

- Add `relative_strength` (RS vs VNINDEX) calculation to backend `IndicatorEngine` and `StrategyIndicatorAdapter`, aligning symbol daily candles with VNINDEX benchmark dates without look-ahead or future data.
- Capture and fixture exact pinned backend column outputs for:
  - `mfi`: `MFI_${length}` (oscillator pane, 0..100 scale, reference lines at 20, 80)
  - `stoch`: `STOCHk_${k}_${d}_${smooth_k}`, `STOCHd_${k}_${d}_${smooth_k}` (oscillator pane, 0..100 scale, reference lines at 20, 80, all-or-nothing multi-series)
  - `adx`: `ADX_${length}`, `DMP_${length}`, `DMN_${length}` (oscillator pane, reference lines at 20, 25, all-or-nothing multi-series)
  - `relative_strength`: `RS_VNINDEX_${length}` or `RS_${length}` (oscillator/indicator subpane, benchmark date-aligned ratio)
- Extend `frontend/src/features/indicators/indicatorDomain.ts` with typed definitions, parameters, default styles, and placement for `mfi`, `stoch`, `adx`, and `relative_strength`.
- Extend `frontend/src/components/chart/IndicatorRenderRegistry.ts` with strict parameter-exact column matching, reference lines, and scales for the new indicators.
- Preserve full lifecycle: add, edit parameters, change color/styles, duplicate, hide, reorder, remove, reload, route navigation, and replay stepping.
- Add backend unit, edge, parity, and benchmark-alignment tests.
- Add frontend unit tests for mapping, exact contracts, and fail-closed behavior on missing multi-series components.
- Extend deterministic Product UAT (`scripts/product-uat.mjs` and manifest) with focused assertions and retained screenshots at 1440×1000 and 1280×800.

## Out of scope

- Keltner Channels, PSAR, SuperTrend (PRO-06).
- Ichimoku Cloud (PRO-07).
- New chart or indicator dependencies.
- Changes to accepted formulas merely to match frontend expectations.
- Market-data provider/sync, trade-planning, strategy-lab, packaging, commit, push, or release work.

## Invariants

- Backend `IndicatorEngine` remains calculation and parameter authority.
- Replay APIs and indicator responses expose no candle or value beyond `current_index`.
- Benchmark alignment for Relative Strength uses only historical VNINDEX candles up to the replay index; never future candles.
- No test or UAT mutates `backend/sumi.db`; record SHA-256 before/after gates.
- Indicator state remains explicit and serializable: identity, definition, parameters, pane, visibility, styles, and order.
- No assertion may be weakened or made non-blocking to obtain green results.

## Target design

### Indicator Catalog Additions

| ID | Label | Placement | Series Keys | Scale / Refs | Parameters |
| --- | --- | --- | --- | --- | --- |
| `mfi` | Money Flow Index | dedicated oscillator | `primary` | 0..100; refs 20, 80 | `length` (default 14) |
| `stoch` | Stochastic Oscillator | dedicated oscillator | `k`, `d` | 0..100; refs 20, 80 | `k` (14), `d` (3), `smooth_k` (3) |
| `adx` | Average Directional Index | dedicated oscillator | `adx`, `dmp`, `dmn` | auto; refs 20, 25 | `length` (14) |
| `relative_strength` | Relative Strength (vs VNINDEX) | dedicated oscillator | `primary` | auto; baseline 0 or 100 | `length` (default 20), `benchmark` ('VNINDEX') |

## Milestones

1. **Backend calculation & Relative Strength contract:** Verify `mfi`, `stoch`, and `adx` outputs in `IndicatorEngine`; implement `relative_strength` with historical VNINDEX date alignment; add unit/parity tests.
2. **Typed released catalog:** Extend `indicatorDomain.ts` types, schemas, default styles, and descriptors for `mfi`, `stoch`, `adx`, and `relative_strength`.
3. **Renderer adapters & tests:** Implement exact parameter-column adapters, multi-series all-or-nothing handling, reference lines, and scales in `IndicatorRenderRegistry.ts`; add focused vitest tests.
4. **Lifecycle & UI integration:** Verify manager dialog, chrome display, subpane allocation, parameter edits, reload, and navigation.
5. **Product UAT & evidence:** Extend `product-uat.mjs` and baseline manifest with PRO-05 assertions; execute full technical gate and Product UAT; inspect screenshots; verify DB hash invariant; update records and stop at Reviewer gate.

## Acceptance mapping

| ID | Required implementation and evidence |
| --- | --- |
| PRO-IND-01 | Registry-driven parameters and backend calculations; parity fixtures for MFI, Stochastic, ADX, Relative Strength. |
| PRO-IND-02 | Exhaustive released catalog with placement, series, format, scale, style, references, warm-up. |
| PRO-IND-03 | Tests proving unknown/engine-only IDs cannot render as EMA or appear released. |
| PRO-IND-04 | Semantic series keys/labels for MFI, Stochastic (K/D), ADX (ADX/+DI/-DI), Relative Strength. |
| PRO-IND-05 | Focused oscillator and multi-series renderer contracts. |
| PRO-IND-06 | Multiple instances and complete manager/persistence/reload/resume lifecycle. |
| PRO-IND-07 | Null/warm-up/gap/replay-boundary fixtures without invalid segments or future data. |
| PRO-IND-08 | Backend parity covers every released output series and representative edges. |
| PRO-IND-09 | Relative Strength aligns symbol and VNINDEX dates explicitly and reports unavailable benchmark coverage honestly. |
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

- 2026-08-15: User authorized PRO-05. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
