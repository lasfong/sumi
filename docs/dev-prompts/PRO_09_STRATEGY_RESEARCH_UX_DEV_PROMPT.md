# PRO-09 DEV Prompt — Strategy Research UX

You are the dedicated DEV session for **PRO-09 — Strategy Research UX**. Implement this batch from the current workspace checkout; do not rely on chat history. Stop at the Independent Reviewer Gate when implementation and verification are complete. Do not approve your own work, commit, push, or start PRO-10.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-09, PRO-STRAT-01..08)
6. `docs/program/PRO_09_STRATEGY_RESEARCH_UX.md`
7. `docs/exec-plans/PRO_09_STRATEGY_RESEARCH_UX.md`

## Outcome

Strategy comparison in Strategy Lab becomes typed, declarative, reproducible, bounded, and resistant to overfitting: typed UI parameter controls (no raw internal JSON editing), non-overlapping In-Sample / Out-of-Sample evaluation periods, bounded parameter sweeps with cancellation support, multi-metric robustness comparison (excluding low-sample or invalid metrics from winning rankings), and unified versioned strategy semantics across Strategy Lab, Scanner, Replay, and Backtest.

## Implementation tasks

1. **Backend Strategy Lab Domain & Validation (`PRO-STRAT-01`, `PRO-STRAT-02`, `PRO-STRAT-03`, `PRO-STRAT-04`):**
   - In `backend/app/services/strategy_lab_service.py` and `backend/app/domain/strategy/`:
     - Provide structured/typed parameter definitions without requiring raw path queries.
     - Validate declarative strategy JSON schemas without using Python `eval` or dynamic code execution.
     - Implement non-overlapping In-Sample (training) vs Out-of-Sample (OOS) date range splitting and record both metrics in result payloads and run manifests.
     - Enforce parameter sweep bounds (e.g. maximum combinations limit) and deterministic execution ordering.
     - Provide cooperative sweep cancellation mechanism.
   - Add unit/integration tests in backend tests (`backend/app/tests/`).

2. **Backend Robustness Scoring & Ranking (`PRO-STRAT-05`, `PRO-STRAT-06`):**
   - Exclude results with insufficient trades (< 5 trades) or invalid statistical metrics from winning top ranking badges, best-variant recommendations, or green highlights on heatmaps.
   - Emit robustness indicators (e.g. OOS stability ratio, profit factor degradation, sample size confidence).

3. **Frontend Strategy Lab UI Enhancements (`PRO-STRAT-01`, `PRO-STRAT-03`, `PRO-STRAT-04`, `PRO-STRAT-06`):**
   - In `frontend/src/pages/StrategyLabPage.tsx` and `frontend/src/api/strategyLabApi.ts`:
     - Replace internal JSON path inputs (`indicators[1].length`) with typed parameter selector controls.
     - Add In-Sample / Out-of-Sample date range inputs with visual validation preventing overlap.
     - Add sweep cancellation button allowing users to cancel long-running grid sweeps.
     - Render comparison table showing In-Sample vs Out-of-Sample metrics, trade count warnings, and robustness score.
   - Add frontend vitest unit tests.

4. **Cross-Workflow Semantic Parity (`PRO-STRAT-07`, `PRO-STRAT-08`):**
   - Ensure strategy schemas and indicator adapter parameters remain 100% compatible across Strategy Lab, Backtest, Replay, and Scanner.

5. **Product UAT & Screenshot Evidence:**
   - Add deterministic UAT checks in `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` for Strategy Lab typed controls, train/test split, robustness ranking exclusion, and sweep cancellation.
   - Retain `pro09-strategy-research-1440x1000.png` and `pro09-strategy-research-1280x800.png`.

6. **Technical Gates & Hand-off:**
   - Run pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_09_STRATEGY_RESEARCH_UX.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-10.
