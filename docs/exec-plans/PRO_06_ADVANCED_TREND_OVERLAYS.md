# PRO-06 — Advanced Trend Overlays

Status: `CLOSED — INDEPENDENTLY APPROVED`

## Outcome

Keltner Channels (`kc`), Parabolic SAR (`psar`), and SuperTrend (`supertrend`) become complete, released product capabilities: backend calculation authority, explicit semantic render definitions as price overlays, channel/marker/direction/trend-color semantics, editable/persistent instances, and browser-proven replay behavior across gaps, nulls, scales, and navigation.

## Context and problem

PRO-05 is independently approved and closed. The backend `IndicatorEngine` already implements `kc`, `psar`, and `supertrend`, but they were unreleased in the frontend. In the frontend, `IndicatorInstanceV1` previously defined 13 released indicators. Keltner Channels, PSAR, and SuperTrend have now been added to the typed catalog with strict parameter-exact output mapping, multi-series all-or-nothing rendering, price overlay placement, direction/color semantics, and full UI/re-render lifecycle.

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
| `psar` | Parabolic SAR | price | `sar` | sar: `#E040FB` | `af0` (0.02), `af` (0.02), `max_af` (0.2) |
| `supertrend` | SuperTrend | price | `supertrend` | supertrend: `#26A69A`, bull: `#26A69A`, bear: `#EF5350` | `length` (7), `multiplier` (3.0) |

## Milestones

1. **Backend calculation & Adapter support:** Verified `kc`, `psar`, and `supertrend` outputs in `IndicatorEngine`; added aliases; updated `StrategyIndicatorAdapter._params_for` and `IndicatorConfig`; added unit/parity tests in `test_indicators.py` and `test_indicator_parity_e2e.py`.
2. **Typed released catalog:** Extended `indicatorDomain.ts` types, schemas, default styles, and descriptors for `kc`, `psar`, and `supertrend`.
3. **Renderer adapters & tests:** Implemented exact parameter-column adapters, `formatFloatParam`, all-or-nothing multi-series handling, and price overlay mapping in `IndicatorRenderRegistry.ts`; added focused vitest tests in `IndicatorRenderRegistry.test.ts` and `indicatorDomain.test.ts`.
4. **Lifecycle & UI integration:** Updated `IndicatorManager.tsx` empty-state copy; verified manager dialog, price overlay chrome display, parameter edits, reload, and navigation.
5. **Product UAT & evidence:** Extended `product-uat.mjs` and baseline manifest with PRO-06 assertions; executed full technical gate and Product UAT (320/320 passed); inspected screenshots (1440×1000 and 1280×800); verified DB hash invariant; updated records and stopped at Reviewer gate.

## Acceptance mapping

| ID | Required implementation and evidence | Status |
| --- | --- | --- |
| PRO-IND-01 | Registry-driven parameters and backend calculations; parity fixtures for Keltner Channels, PSAR, SuperTrend. | PASS |
| PRO-IND-02 | Exhaustive released catalog with placement, series, format, scale, style, references, warm-up. | PASS |
| PRO-IND-03 | Tests proving unknown/engine-only IDs cannot render as EMA or appear released. | PASS |
| PRO-IND-04 | Semantic series keys/labels for Keltner Channels (`upper`/`middle`/`lower`), PSAR, SuperTrend. | PASS |
| PRO-IND-05 | Focused overlay, channel, and trend-color renderer contracts. | PASS |
| PRO-IND-06 | Multiple instances and complete manager/persistence/reload/resume lifecycle. | PASS |
| PRO-IND-07 | Null/warm-up/gap/replay-boundary fixtures without invalid segments or future data. | PASS |
| PRO-IND-08 | Backend parity covers every released output series and representative edges. | PASS |
| PRO-IND-11 | Retained browser evidence for labels, values, scales, panes, settings, persistence, and navigation. | PASS |

## Verification evidence

- **Database Invariant**:
  - `backend/sumi.db` SHA-256 before/after: `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080` (0 bytes mutated).
- **Fast Technical Gate (`verify-v2.ps1`)**:
  - Backend pytest: 159 passed (0 failed).
  - Alembic migrations: clean (0 drift).
  - ESLint: 0 errors.
  - Frontend vitest: 179 passed across 27 files (51 passed across 6 indicator/chart files).
  - Frontend production build (`tsc -b && vite build`): clean (0 errors).
- **Product UAT Verification (`run-product-uat.ps1`)**:
  - Directory: `test-results/product-uat/2026-08-15T16-26-02-855Z/`
  - `results.json` SHA-256: `C98FAD635E1966522CB250892BAC9D1FAC327C8A4CB276902D412E8851E7223F`
  - Assertions Passed: **320 / 320** (0 failed, 0 blocking failed).
  - Manifest Reconciliation: `pass: true` (8/8 tests passed in `scripts/product-uat-manifest.test.mjs`).
  - PRO-06 Assertions:
    - `pro06.kc-channel`: PASS (renderedValues: {upper: 95120.25, middle: 87497.58, lower: 79874.91}, expectedValues: {upper: 95120.25, middle: 87497.58, lower: 79874.91})
    - `pro06.psar-overlay`: PASS (renderedValue: 87495.39, expectedValue: 87495.39)
    - `pro06.supertrend-overlay`: PASS (renderedValue: 78865.02, expectedValue: 78865.02)
    - `pro06.advanced-trend-lifecycle`: PASS (all instances verified in DOM and runtime)
- **Retained Visual Screenshots**:
  - `pro06-advanced-trend-indicators-1440x1000.png` (1440×1000, SHA-256: `2B2954EE1CA692E6A87269D94379D81DAD3D6575F6F3160DF33F09380697694A`)
  - `pro06-advanced-trend-indicators-1280x800.png` (1280×800, SHA-256: `FC12333800DE50F7840504865B4DCDBCEB4AE156FF0FF0E730F8D694D683AE11`)
- **Whitespace / Format Check**:
  - `git diff --check`: 0 errors.

## Progress log

- 2026-08-15: User authorized PRO-06. Reviewer prepared ExecPlan and standalone DEV prompt.
- 2026-08-15: DEV completed implementation of Keltner Channels, Parabolic SAR, and SuperTrend. All unit tests, technical gate, and Product UAT passed (320/320). Batch reached Independent Reviewer Gate.
- 2026-08-16: Independent Reviewer audited implementation, contracts, and test evidence. Verdict: `APPROVE` recorded in `docs/reviews/PRO_06_REVIEW_2026-08-16.md`. PRO-06 is closed; PRO-07 remains unauthorized.
