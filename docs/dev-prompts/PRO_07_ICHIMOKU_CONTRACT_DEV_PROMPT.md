# PRO-07 DEV Prompt — Ichimoku Contract

You are the dedicated DEV session for **PRO-07 — Ichimoku Contract**.  Implement this batch from the current workspace checkout; do not rely on chat history.  Stop at the Independent Reviewer Gate when implementation and verification are complete.  Do not approve your own work, commit, push, or start PRO-08.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-07, PRO-IND-01..11, especially PRO-IND-10)
6. `docs/program/PRO_07_ICHIMOKU_CONTRACT.md`
7. `docs/exec-plans/PRO_07_ICHIMOKU_CONTRACT.md`

## Outcome

Release Ichimoku Cloud (`ichimoku`) as a full product capability with backend calculation authority, strict no-look-ahead displacement contract, parameter-exact column mapping, semantic series keys (`tenkan`, `kijun`, `spanA`, `spanB`, `chikou`), price overlay placement, and browser-verified replay behavior across start/end boundaries, gaps, warm-up, replay stepping, rewind, and reload.

## Implementation tasks

1. **Backend Indicator Calculation & Strategy Adapter Support:**
   - In `backend/app/domain/engine/indicator_engine.py`:
     - Verify `ichimoku` calculations and exact output column formats: `ITS_${tenkan}`, `IKS_${kijun}`, `ISA_${tenkan}`, `ISB_${kijun}`, `ICS_${kijun}`.
   - In `backend/app/domain/engine/strategy_indicator_adapter.py` & `backend/app/domain/strategy/strategy_schema.py`:
     - Add parameters: `tenkan`, `kijun`, `senkou` to `StrategyIndicatorAdapter._params_for` and `IndicatorConfig`.
   - Add unit, edge, and parity tests in `backend/app/tests/test_indicators.py` and `backend/app/tests/test_indicator_parity_e2e.py`.

2. **Frontend Typed Catalog & Domain:**
   - In `frontend/src/features/indicators/indicatorDomain.ts`:
     - Add `'ichimoku'` to `SUPPORTED_INDICATORS` and `IndicatorInstanceV1['definitionId']`.
     - Define default parameters (`tenkan`: 9, `kijun`: 26, `senkou`: 52), descriptor metadata, category (`Trend`), placement (`price`), and default styles (`tenkan`: `#26A69A`, `kijun`: `#EF5350`, `spanA`: `#00E5FF`, `spanB`: `#FF8A00`, `chikou`: `#E040FB`).
     - Update validation schemas and tests in `indicatorDomain.test.ts`.

3. **Frontend Semantic Rendering & Ichimoku Price Overlay Adapter:**
   - In `frontend/src/components/chart/IndicatorRenderRegistry.ts`:
     - Implement exact column matching and all-or-nothing multi-series mapping for `ichimoku`:
       - `tenkan`: `ITS_${tenkan}`
       - `kijun`: `IKS_${kijun}`
       - `spanA`: `ISA_${tenkan}`
       - `spanB`: `ISB_${kijun}`
       - `chikou`: `ICS_${kijun}`
     - Enforce the no-look-ahead displacement contract (PRO-IND-10): display points map strictly to timestamp rows returned from the backend (which never exceed `current_index`).
     - Add unit tests in `frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts` covering exact column contracts, all-or-nothing rendering, and rejection of partial/mismatched payloads.

4. **Product UAT & Screenshot Evidence:**
   - Update `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` to exercise and assert Ichimoku Cloud against session-scoped backend calculation outputs on the price pane.
   - Retain `pro07-ichimoku-cloud-1440x1000.png` and `pro07-ichimoku-cloud-1280x800.png`.

5. **Technical Gates & Hand-off:**
   - Run focused pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_07_ICHIMOKU_CONTRACT.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-08.
