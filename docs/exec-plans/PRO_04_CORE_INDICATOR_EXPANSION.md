# PRO-04 — Core Indicator Expansion

Status: `PREPARED — NOT STARTED`

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

## Decision log

- Keep backend `IndicatorEngine` authoritative; PRO-04 releases existing definitions rather than reimplementing formulas in TypeScript.
- Use an exhaustive Sumi-owned product catalog; the backend registry may contain future engine-only definitions.
- Treat raw Volume and backend Volume SMA as separate definitions sharing a pane, not interchangeable render modes.

## Completion evidence

Not started. DEV must append changed-file inventory, exact test counts, result/artifact paths, screenshot dimensions and hashes, manifest reconciliation, DB before/after hashes, cleanup evidence, deviations, and reviewer checklist before handoff.
