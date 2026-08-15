# PRO-05 DEV Prompt — Momentum and Relative Strength

You are the dedicated DEV session for **PRO-05 — Momentum and Relative Strength**.  Implement this batch from the current workspace checkout; do not rely on chat history.  Stop at the Independent Reviewer Gate when implementation and verification are complete.  Do not approve your own work, commit, push, or start PRO-06.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-05, PRO-IND-01..09, PRO-IND-11)
6. `docs/program/PRO_05_MOMENTUM_AND_RELATIVE_STRENGTH.md`
7. `docs/exec-plans/PRO_05_MOMENTUM_AND_RELATIVE_STRENGTH.md`

## Outcome

Release MFI, Stochastic Oscillator, ADX, and Relative Strength versus VNINDEX (RS-VNINDEX) as full product capabilities with backend calculation authority, explicit benchmark alignment, parameter-exact column mapping, all-or-nothing multi-series rendering, reference lines, oscillator subpane management, and browser-verified replay behavior.

## Implementation tasks

1. **Backend Indicator Calculation & Relative Strength Contract:**
   - In `backend/app/domain/engine/indicator_engine.py`:
     - Verify `mfi`, `stoch`, `adx` calculations and their exact column outputs.
     - Implement `relative_strength` indicator calculation (ratio of symbol price performance to VNINDEX benchmark over lookback window `length`, aligning dates strictly up to current replay index without future candles).
   - In `backend/app/domain/engine/strategy_indicator_adapter.py`:
     - Support parameters for `mfi`, `stoch` (`k`, `d`, `smooth_k`), `adx`, and `relative_strength`.
   - Add unit, edge, and parity tests in `backend/app/tests/test_indicators.py` and `backend/app/tests/test_indicator_parity_e2e.py`.

2. **Frontend Typed Catalog & Domain:**
   - In `frontend/src/features/indicators/indicatorDomain.ts`:
     - Add `'mfi'`, `'stoch'`, `'adx'`, `'relative_strength'` to `SUPPORTED_INDICATORS` and `IndicatorInstanceV1['definitionId']`.
     - Define default parameters, descriptor metadata, categories, placements (`oscillator`), and default styles.
     - Update validation schemas and tests in `indicatorDomain.test.ts`.

3. **Frontend Semantic Rendering & Multi-Series Adapters:**
   - In `frontend/src/components/chart/IndicatorRenderRegistry.ts`:
     - Implement exact column matching and all-or-nothing multi-series mapping for:
       - `mfi`: `MFI_${length}`, scale `0..100`, reference lines at 20, 80.
       - `stoch`: `k` series (`STOCHk_${k}_${d}_${smooth_k}`) and `d` series (`STOCHd_${k}_${d}_${smooth_k}`), scale `0..100`, reference lines at 20, 80.
       - `adx`: `adx` series (`ADX_${length}`), `dmp` series (`DMP_${length}`), `dmn` series (`DMN_${length}`), reference lines at 20, 25.
       - `relative_strength`: `primary` series, benchmark comparison vs VNINDEX.
     - Add unit tests in `frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts` covering exact column contracts, all-or-nothing rendering, and rejection of partial/mismatched payloads.

4. **Product UAT & Screenshot Evidence:**
   - Update `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` to exercise and assert MFI, Stochastic, ADX, and Relative Strength against session-scoped backend calculation outputs.
   - Retain `pro05-momentum-indicators-1440x1000.png` and `pro05-momentum-indicators-1280x800.png`.

5. **Technical Gates & Hand-off:**
   - Run focused pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_05_MOMENTUM_AND_RELATIVE_STRENGTH.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-06.
