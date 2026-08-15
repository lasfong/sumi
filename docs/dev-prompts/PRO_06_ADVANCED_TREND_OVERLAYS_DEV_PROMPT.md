# PRO-06 DEV Prompt — Advanced Trend Overlays

You are the dedicated DEV session for **PRO-06 — Advanced Trend Overlays**.  Implement this batch from the current workspace checkout; do not rely on chat history.  Stop at the Independent Reviewer Gate when implementation and verification are complete.  Do not approve your own work, commit, push, or start PRO-07.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-06, PRO-IND-01..08, PRO-IND-11)
6. `docs/program/PRO_06_ADVANCED_TREND_OVERLAYS.md`
7. `docs/exec-plans/PRO_06_ADVANCED_TREND_OVERLAYS.md`

## Outcome

Release Keltner Channels (`kc`), Parabolic SAR (`psar`), and SuperTrend (`supertrend`) as full product capabilities with backend calculation authority, parameter-exact column mapping, channel/marker/trend-direction semantics, price overlay placement, and browser-verified replay behavior.

## Implementation tasks

1. **Backend Indicator Calculation & Strategy Adapter Support:**
   - In `backend/app/domain/engine/indicator_engine.py`:
     - Verify `kc`, `psar`, `supertrend` calculations and exact output column formats.
     - Note column name patterns:
       - `kc`: `KCUe_${length}_${scalarStr}`, `KCBe_${length}_${scalarStr}`, `KCLe_${length}_${scalarStr}`
       - `psar`: `PSARl_${af0}_${max_af}`, `PSARs_${af0}_${max_af}`, etc.
       - `supertrend`: `SUPERT_${length}_${multiplierStr}`, `SUPERTd_${length}_${multiplierStr}`, etc.
   - In `backend/app/domain/engine/strategy_indicator_adapter.py` & `backend/app/domain/strategy/strategy_schema.py`:
     - Add parameters: `scalar`, `af0`, `af`, `max_af`, `multiplier`.
   - Add unit, edge, and parity tests in `backend/app/tests/test_indicators.py` and `backend/app/tests/test_indicator_parity_e2e.py`.

2. **Frontend Typed Catalog & Domain:**
   - In `frontend/src/features/indicators/indicatorDomain.ts`:
     - Add `'kc'`, `'psar'`, `'supertrend'` to `SUPPORTED_INDICATORS` and `IndicatorInstanceV1['definitionId']`.
     - Define default parameters, descriptor metadata, categories, placements (`price`), and default styles.
     - Update validation schemas and tests in `indicatorDomain.test.ts`.

3. **Frontend Semantic Rendering & Price Overlay Adapters:**
   - In `frontend/src/components/chart/IndicatorRenderRegistry.ts`:
     - Implement exact column matching and all-or-nothing multi-series mapping for:
       - `kc`: `upper` (`KCUe...`), `middle` (`KCBe...`), `lower` (`KCLe...`) price overlay channels.
       - `psar`: `sar` points/markers price overlay.
       - `supertrend`: `supertrend` price overlay line with trend direction coloring.
     - Format floats/scalars deterministically (`scalarStr`, `multiplierStr`, etc.) avoiding precision mismatch defects.
     - Add unit tests in `frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts` covering exact column contracts, all-or-nothing rendering, and rejection of partial/mismatched payloads.

4. **Product UAT & Screenshot Evidence:**
   - Update `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` to exercise and assert Keltner Channels, PSAR, and SuperTrend against session-scoped backend calculation outputs on the price pane.
   - Retain `pro06-advanced-trend-indicators-1440x1000.png` and `pro06-advanced-trend-indicators-1280x800.png`.

5. **Technical Gates & Hand-off:**
   - Run focused pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_06_ADVANCED_TREND_OVERLAYS.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-07.
