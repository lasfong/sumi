# PRO-04 — Core Indicator Expansion

Status: `CLOSED — INDEPENDENTLY APPROVED (R5 APPROVE)`

## Outcome

SMA, Bollinger Bands, ATR, and backend Volume SMA become complete product capabilities: correct backend values, explicit semantic render definitions, correct overlay/pane placement, editable/persistent instances, and browser-proven replay behavior. Raw candle Volume remains a distinct product definition from backend-calculated Volume SMA.

## Context and problem

PRO-03 is independently approved. The backend `IndicatorEngine` already registers SMA, Bollinger Bands (`bbands`), ATR, and `volume_sma`, but backend availability is not product release. The frontend currently approves only EMA, RSI, MACD, CCI, and raw Volume. `IndicatorInstanceV1` is a closed union for those five definitions, and `IndicatorRenderRegistry.mapBackendData()` falls through unknown single-series data to an EMA label/renderer. That fallback violates PRO-IND-02/03/04.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs PRO-IND-01..08 and PRO-IND-11; `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`; V3 G-01..05 and R-01 regression.

## In scope

- Define an exhaustive frontend released-definition catalog for EMA, RSI, MACD, CCI, raw Volume, SMA, Bollinger Bands, ATR, and Volume SMA.
- Remove generic EMA fallback behavior; unknown or engine-only definitions fail closed and remain unavailable in the product UI.
- Map backend outputs to semantic series keys and user labels without exposing dataframe column names.
- Render SMA as a price overlay, Bollinger Bands as upper/middle/lower price overlays, ATR in its own oscillator pane, raw Volume as candle-volume histogram, and Volume SMA as a calculated line in the volume pane.
- Preserve multiple instances, parameters, visibility, style, ordering, reload, route navigation, and replay resume for all newly released definitions.
- Add backend parity/edge fixtures for every new series, including warm-up/null/gap/no-future boundaries.
- Add focused frontend domain, repository, render-registry, pane/series, and manager tests.
- Extend deterministic Product UAT with focused assertions and retained screenshots at 1440×1000 and 1280×800.

## Out of scope

- MFI, Stochastic, ADX, Relative Strength, Keltner, PSAR, SuperTrend, and Ichimoku (PRO-05..07).
- New chart, drawing, or indicator dependencies.
- Changes to accepted formulas merely to match frontend expectations.
- Market-data provider/sync, trading-plan, strategy-lab, packaging, commit, push, tag, or release work.

## Invariants

- Backend `IndicatorEngine` remains calculation/parameter authority.
- Replay APIs and indicator responses expose no candle/value beyond `current_index`.
- No test or UAT mutates `backend/sumi.db`; record SHA-256 before/after gates.
- Indicator state remains explicit and serializable: identity, definition, parameters, pane, visibility, styles, and order.
- No assertion may be weakened or made non-blocking to obtain green results.
- Preserve the current dirty workspace and do not include PRO-05 work.

## Current architecture

- Backend registry/calculation: `backend/app/domain/engine/indicator_engine.py`.
- Backend API: `backend/app/api/indicators.py` and replay integration in `backend/app/api/replay.py`.
- Backend coverage: `backend/app/tests/test_indicators.py`, `backend/app/tests/test_indicator_parity_e2e.py`.
- Frontend API types: `frontend/src/api/indicatorsApi.ts`.
- Product state/persistence: `frontend/src/features/indicators/indicatorDomain.ts`, `IndicatorRepository.ts`.
- Renderer mapping: `frontend/src/components/chart/IndicatorRenderRegistry.ts`.
- Pane/series lifecycle: `PaneManager.ts`, `SeriesManager.ts`, `CandleChart.tsx`.
- User management surface: `IndicatorManager.tsx`.
- Product UAT authority: `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json`.

## Target design

Create a typed, exhaustive released-definition catalog keyed by product definition ID. Each entry owns placement, semantic series keys, user labels, renderer type, styles, formatter/scale, reference lines, and warm-up policy. Backend response columns are resolved inside definition-specific adapters; raw dataframe labels never become UI semantics. The catalog must make unsupported IDs an explicit error/unsupported state, never an EMA fallback.

Expected semantic mappings:

| Definition | Placement | Semantic series |
| --- | --- | --- |
| `sma` | price | `average` |
| `bbands` | price | `upper`, `middle`, `lower` |
| `atr` | dedicated oscillator | `atr` |
| `volume` | volume pane | `raw-volume` histogram from visible candles |
| `volume_sma` | volume pane | `average-volume` line from backend calculation |

The implementation must discover and fixture the exact backend column outputs produced by the pinned `pandas-ta`; do not match arbitrary “first column” data. If persisted indicator schema evolution is required, add an explicit version/migration preserving valid V1 documents.

## Milestones

1. **Contract inventory and red tests:** record exact backend output columns and warm-up behavior; add failing backend/frontend tests for the four new definitions and unknown-definition fail-closed behavior.
2. **Typed released catalog:** extend domain types/default styles/persistence validation and make released/engine-only definitions explicit and exhaustive.
3. **Renderer completion:** implement semantic mappings, pane placement, series styles/scales, null/gap handling, and raw Volume versus Volume SMA coexistence.
4. **Workflow completion:** prove add/edit/duplicate/hide/reorder/remove/reload/resume/navigation and replay stepping for every new definition.
5. **Verification and evidence:** run focused/full gates, deterministic Product UAT, inspect screenshots, reconcile manifest, confirm DB/process cleanup, update this plan and ledger, and stop at Reviewer gate.

## Acceptance mapping

| ID | Required implementation and evidence |
| --- | --- |
| PRO-IND-01 | Registry-driven parameters and backend calculations; parity fixtures. |
| PRO-IND-02 | Exhaustive released catalog with placement, series, format, scale, style, references, warm-up. |
| PRO-IND-03 | Tests proving unknown/engine-only IDs cannot render as EMA or appear released. |
| PRO-IND-04 | Semantic series keys/labels for SMA, bands, ATR, and Volume SMA. |
| PRO-IND-05 | Focused overlay, channel, oscillator, histogram/volume-line renderer contracts. |
| PRO-IND-06 | Multiple instances and complete manager/persistence/reload/resume lifecycle. |
| PRO-IND-07 | Null/warm-up/gap/replay-boundary fixtures without invalid segments or future data. |
| PRO-IND-08 | Backend parity covers every released output series and representative edges. |
| PRO-IND-11 | Retained browser evidence for labels, values, scales, panes, settings, persistence, and navigation. |

## Verification commands

Run from repository root unless noted. Use the repository's Windows wrappers on Windows.

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py -q
Set-Location ..\frontend
npm.cmd test -- --run src/features/indicators src/components/chart/__tests__/IndicatorRenderRegistry.test.ts src/components/chart/__tests__/SeriesManager.test.ts src/components/chart/__tests__/PaneManager.test.ts
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

The full product wrapper is `scripts/verify-product.sh`; on Windows it may require WSL because it expects `.venv/bin/python`. If it cannot run directly, record that platform fact and use the repository-supported Windows constituents (`verify-v2.ps1` and `run-product-uat.ps1`); do not invent or weaken a gate. Product UAT must retain results, manifest reconciliation, runtime errors, and PRO-04 screenshots at both required viewport sizes.

## Rollback and compatibility

- Prefer additive frontend definition/catalog changes with explicit persistence migration if the document schema changes.
- Existing EMA/RSI/MACD/CCI/raw Volume documents must remain readable and behaviorally unchanged.
- Removing the newly released definitions from the approved catalog is the product rollback; persisted unknown instances must fail safely without corrupting the document.
- No production database migration is expected. If one becomes necessary, stop for reviewer re-planning.

## Risks and mitigations

- Pinned `pandas-ta` column names vary by parameters: capture exact outputs in backend fixtures and map them inside definition-specific adapters.
- Generic fallback mislabels data: enforce exhaustive switches/maps and `never`/explicit unsupported tests.
- Bollinger series ordering ambiguity: match semantic upper/middle/lower evidence, never array position alone.
- Volume SMA confused with raw Volume: use separate IDs, visual types, labels, data sources, and coexistence UAT.
- Warm-up nulls collapse into zero or connecting segments: filter invalid points and assert first valid timestamps.
- More overlays/panes destabilize lifecycle: test replay, rewind, pan/zoom, reload, route navigation, and unmount cleanup.

## Progress log

- 2026-08-10: Independent Reviewer prepared this complete plan for machine transfer. PRO-04 implementation has not started.
- 2026-08-12: User authorized PRO-04. Await DEV startup gate; product implementation has not started.
- 2026-08-12: Complete initial execution of PRO-04 batch across backend calculation parity, typed catalog expansion, rendering adapters, volume pane line coexistence, unit tests, fast technical gate (`verify-v2.ps1`), and deterministic Product UAT (`run-product-uat.ps1`).
- 2026-08-13: Independent Reviewer R1 returned PRO-04 for REWORK-01. Required correction: explicit exact-column adapters and one chrome group per physical pane.
- 2026-08-13: Executed REWORK-01. Replaced generic `available[0]` with `findSemanticColumn`. Refactored `IndicatorPaneChrome.tsx` to group visible non-price instances by physical `paneId` so each physical subpane receives exactly one flex section container. Added unit tests in `IndicatorRenderRegistry.test.ts` and `IndicatorPaneChrome.test.tsx`.
- 2026-08-13: Independent Reviewer R2 returned PRO-04 for REWORK-02. REWORK-01 fixed shared-pane chrome alignment, but `findSemanticColumn` retained parameter-mismatched prefix fallbacks (`startsWith`). REWORK-02 requires parameter-exact matching for all released definitions (EMA, RSI, MACD, CCI, SMA, BBands, ATR, Volume SMA), unit tests covering parameter mismatch cases, UAT parameter assertions, 1280x800 scroll capture, and ExecPlan prose consolidation.
- 2026-08-13: Executed REWORK-02. Replaced prefix matching in `IndicatorRenderRegistry.ts` with strict parameter-exact column selection (`getIntParam`/`getFloatParam`). Added regression tests in `IndicatorRenderRegistry.test.ts` for parameter mismatches across length, std, and fast/slow/signal. Strengthened `product-uat.mjs` with parameter contract assertions and scrolled ATR pane into view for the 1280x800 screenshot. Consolidated `PRO_04_CORE_INDICATOR_EXPANSION.md`. Re-verified technical gate (`verify-v2.ps1`) and deterministic Product UAT (`run-product-uat.ps1`).
- 2026-08-13: Independent Reviewer R3 returned PRO-04 for REWORK-03. R2 removed generic parameter-prefix fallback, and R1 layout is correct, but CCI/ATR/Bollinger still accept aliases/alternate formats and MACD/Bollinger render partial responses. REWORK-03 must enforce one pinned output contract per definition, all-or-nothing multi-series rendering, and UAT equality against scoped backend output values. Authority: `docs/reviews/PRO_04_REVIEW_2026-08-13_R3.md` and `docs/dev-prompts/PRO_04_REWORK_03_ALL_OR_NOTHING_OUTPUT_PROMPT.md`.
- 2026-08-13: Executed REWORK-03. Enforced single pinned output contracts for CCI (`CCI_${length}_0.015`), ATR (`ATRr_${length}`), and Bollinger (`BBU_${length}_${stdStr}_${stdStr}`) in `IndicatorRenderRegistry.ts` (removed aliases). Implemented all-or-nothing rendering for MACD and Bollinger Bands (returning `[]` if any component series is missing). Added unit tests in `IndicatorRenderRegistry.test.ts` rejecting aliases, alternate spellings, and partial payloads. Updated `scripts/product-uat.mjs` to fetch session-scoped backend indicator data and assert rendered values match exact expected backend column values. Re-verified fast gate (`verify-v2.ps1`) and Product UAT (`run-product-uat.ps1`).
- 2026-08-15: Independent Reviewer R4 returned PRO-04 for REWORK-04 (P1). The public valid `bbands.std` parameter is silently ignored by pinned `pandas-ta` because the library requires `lower_std`/`upper_std`; the frontend's `toFixed(1)` expected name is also wrong for fractional values. Authority: `docs/reviews/PRO_04_REVIEW_2026-08-15_R4.md` and `docs/dev-prompts/PRO_04_REWORK_04_BBANDS_STD_CONTRACT_PROMPT.md`.
- 2026-08-15: Executed REWORK-04. Translated public bbands `std` parameter to `lower_std`/`upper_std` in `IndicatorEngine` and `StrategyIndicatorAdapter`, implemented `formatBollingerStd` in `IndicatorRenderRegistry.ts` for exact fractional and integral float output column formatting, added backend/frontend/parity tests for non-default `std=2.25` and `std=1.15`, and updated `product-uat.mjs` to assert non-default bbands (`std=2.25`) against exact scoped backend columns. Re-verified technical gate (`verify-v2.ps1`), product gate (`verify-product.sh`), and deterministic Product UAT (`run-product-uat.ps1`).
- 2026-08-15: Independent Reviewer R5 audited REWORK-04 and confirmed end-to-end resolution. Issued `APPROVE` verdict in `docs/reviews/PRO_04_REVIEW_2026-08-15_R5.md`. PRO-04 is closed; PRO-05 remains unauthorized.

## Decision log

- Keep backend `IndicatorEngine` authoritative; PRO-04 releases existing definitions rather than reimplementing formulas in TypeScript.
- Use an exhaustive Sumi-owned product catalog; the backend registry may contain future engine-only definitions.
- Treat raw Volume and backend Volume SMA as separate definitions sharing a pane, not interchangeable render modes.

## Completion evidence

- Status: `CLOSED — INDEPENDENTLY APPROVED (R5 APPROVE)`
- Reviewer Record: `docs/reviews/PRO_04_REVIEW_2026-08-15_R5.md`
- Changed Files:
  - `backend/app/domain/engine/indicator_engine.py`
  - `backend/app/domain/engine/strategy_indicator_adapter.py`
  - `backend/app/domain/strategy/strategy_schema.py`
  - `backend/app/tests/test_indicators.py`
  - `backend/app/tests/test_indicator_parity_e2e.py`
  - `frontend/src/components/chart/IndicatorRenderRegistry.ts`
  - `frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts`
  - `scripts/product-uat.mjs`
  - `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
  - `docs/AUTONOMOUS_EXECUTION_STATE.md`
- Database Hash Invariant:
  - SHA-256 Before: `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`
  - SHA-256 After: `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080` (0 bytes mutated)
- Technical Gate Verification:
  - Pytest (`test_indicators.py`, `test_indicator_parity_e2e.py`): 13 passed (1.02s)
  - Vitest indicator suite (6 files): 43 passed (1.96s)
  - `verify-v2.ps1` fast technical gate: 148 backend tests passed, Alembic migrations clean, ESLint clean (0 errors), 171 frontend tests passed across 27 files, frontend production build clean (`tsc -b && vite build`)
- Product UAT Verification:
  - Directory: `test-results/product-uat/2026-08-15T14-13-42-897Z/`
  - `results.json` SHA-256: `227C60A3E18C8B01467B0AC8D5CEA1AA7C5A73F9A760D6E690521C800E9C3E26`
  - Assertions Passed: **311 / 311**
  - Assertions Failed: **0**
  - Blocking Failures: **0**
  - Manifest Reconciliation: `pass: true`
  - Runtime Errors: 0
  - Provider Errors: 0
- Retained Visual Screenshots:
  - `pro04-core-indicators-1440x1000.png` (1440×1000, 181,929 bytes, SHA-256: `A7E3E0570FC391CD4B68C53CBE2EE3E6FC21EF3B847E8CF3067EB7779F9486B6`)
  - `pro04-core-indicators-1280x800.png` (1280×800, 155,488 bytes, SHA-256: `1FACAA1A6BD51E64F0572EF5FDECDD57C24315C2FED97474103CC734D3E1E3B0`)
- Whitespace Check:
  - `git diff --check`: 0 errors
