# PRO-09 — Strategy Research UX

Status: `CLOSED — INDEPENDENTLY APPROVED`

## Outcome

Strategy comparison in Strategy Lab becomes typed, declarative, reproducible, bounded, and resistant to overfitting: typed UI parameter controls (no raw internal JSON editing), non-overlapping In-Sample / Out-of-Sample evaluation periods, bounded parameter sweeps with cancellation support, multi-metric robustness comparison (excluding low-sample or invalid metrics from winning rankings), and unified versioned strategy semantics across Strategy Lab, Scanner, Replay, and Backtest.

## Context and problem

PRO-08 is independently approved and closed. Previously, Strategy Lab supported baseline grid sweeps and heatmaps, but users still faced risks of overfitting, lack of explicit In-Sample / Out-of-Sample (OOS) validation splits, untyped parameter paths, sweep execution timeouts without cancellation guarantees, and misleading metrics winning top ranks when sample counts were insufficient.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs `PRO-STRAT-01` through `PRO-STRAT-08` and `PRO-BT` regression; `docs/program/PRO_09_STRATEGY_RESEARCH_UX.md`; V3 G-01..05 regression.

## In scope

1. **Typed Strategy Controls & Validation (`PRO-STRAT-01`, `PRO-STRAT-02`):**
   - Typed parameter controls in Strategy Lab UI: structured dropdowns/inputs for indicator params (length, multiplier, etc.) without exposing internal paths like `indicators[0].length`.
   - Declarative, versioned strategy validation in backend without Python `eval` or arbitrary code execution.
2. **In-Sample vs Out-of-Sample (OOS) Evaluation (`PRO-STRAT-03`):**
   - Explicit training (In-Sample) and testing (Out-of-Sample) non-overlapping date ranges retained in run manifests.
   - Distinct In-Sample and Out-of-Sample performance metrics display (PnL, Win Rate, Profit Factor, Max Drawdown, Sample Size).
3. **Bounded Parameter Sweeps & Cancellation (`PRO-STRAT-04`):**
   - Configurable sweep parameter ranges with safety bounds (max combinations cap: 1..50) and deterministic execution ordering.
   - Cooperative sweep cancellation mechanism (`POST /api/strategy-lab/sweep/cancel`) allowing users to abort running sweeps cleanly.
4. **Robustness Comparison & Ranking Exclusion (`PRO-STRAT-05`, `PRO-STRAT-06`):**
   - Comparison surface displaying sample size, metric validity, robustness score, and OOS stability rather than only peak PnL.
   - Low-sample (< 5 trades) or invalid/insufficient observations automatically excluded from winning ranking badges or heatmap recommendations.
5. **Cross-Workflow Strategy Parity & Reproducibility (`PRO-STRAT-07`, `PRO-STRAT-08`):**
   - Strategy definitions, indicators, and execution semantics shared cleanly across Strategy Lab, Backtest, Scanner, and Replay.
   - Run reproduction verification ensuring identical inputs yield identical outputs.
6. **Automated Testing & Browser Evidence:**
   - Backend pytest unit/integration tests for sweep limits, train/test split, robustness scoring, and cancellation.
   - Frontend vitest tests for typed strategy form, OOS metrics rendering, and ranking displays.
   - Deterministic Product UAT assertions (`pro09.*`) and retained 1440×1000 and 1280×800 screenshots.

## Out of scope

- Market data provider licensing/sync (PRO-10, PRO-11).
- Release candidate bundling & publication (PRO-12).

## Invariants

- Strategy evaluation remains declarative: no `eval` or dynamic code execution in backend.
- Local-first behavior: no user strategy or backtest results transmitted externally.
- `backend/sumi.db` SHA-256 remains untouched during tests/UAT.
- No accepted metric or rule is weakened to achieve green verification.

## Milestones

1. **Backend domain & APIs:** Extend `strategy_lab_service.py`, `strategy_schema.py`, and endpoints for In-Sample/Out-of-Sample date splitting, bounded sweep generation, cooperative cancellation, and robustness metrics filtering; add tests in `test_strategy_lab.py`.
2. **Frontend Typed Controls:** Implement typed parameter inputs in `StrategyLabPage.tsx` replacing internal path editing.
3. **In-Sample/OOS UI & Robustness Ranking:** Add In-Sample vs OOS comparison views, sample-size validity filters, and cancellation button in `StrategyLabPage.tsx`.
4. **Cross-Workflow Semantics:** Verify strategy definition compatibility across Strategy Lab, Backtest, Replay, and Scanner.
5. **Product UAT & Verification:** Extend `product-uat.mjs` with PRO-09 strategy research assertions; run fast technical gate and full Product UAT; capture screenshots; stop at Reviewer Gate.

## Acceptance mapping

| ID | Requirement | Status |
| --- | --- | --- |
| PRO-STRAT-01 | Strategy parameters use typed product controls; users do not edit internal paths such as `indicators[1].length`. | Verified in unit test & browser UAT |
| PRO-STRAT-02 | Strategy definitions are versioned declarative data validated without `eval` or arbitrary Python execution. | Verified in unit test & browser UAT |
| PRO-STRAT-03 | Training and out-of-sample periods are explicit, non-overlapping, and retained in the run manifest. | Verified in unit test & browser UAT |
| PRO-STRAT-04 | Parameter sweeps enforce bounds, maximum variants, cancellation, and deterministic ordering. | Verified in unit test & browser UAT |
| PRO-STRAT-05 | Invalid/insufficient metrics cannot win ranking, heatmap, comparison, or recommendation surfaces. | Verified in unit test & browser UAT |
| PRO-STRAT-06 | Comparison shows coverage, sample size, assumptions, robustness, and out-of-sample results rather than only maximum PnL. | Verified in unit test & browser UAT |
| PRO-STRAT-07 | Saved strategy versions and runs can be reproduced or report why required data/version is unavailable. | Verified in unit test & browser UAT |
| PRO-STRAT-08 | Scanner, Replay, Backtest, and Strategy Lab share the same versioned strategy and indicator semantics. | Verified in unit test & browser UAT |

## Verification commands & results

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
# SHA256: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A (unchanged)

Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
# 176 passed in 5.19s

Set-Location ..\frontend
npm.cmd test -- --run
# 27 test files passed, 182 tests passed

npm.cmd run lint; npm.cmd run build
# ESLint clean (0 errors), Vite production build clean

Set-Location ..
.\scripts\verify-v2.ps1
# All fast technical gates passed

.\scripts\run-product-uat.ps1
# 333 passed, 0 failed, 0 blocking failed (Reconciliation: pass)

git diff --check
# Clean, no trailing whitespace or conflicts

Get-FileHash -Algorithm SHA256 backend\sumi.db
# SHA256: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A (unchanged)
```

## Retained Visual Evidence

- `pro09-strategy-research-1440x1000.png` (203,605 bytes, SHA-256: `66d8f8a846175ecfbf8da4fe8f64560731f2ae5ba312984578b548b8987b22ff`)
- `pro09-strategy-research-1280x800.png` (165,517 bytes, SHA-256: `81bda3bfd82cefd211df051663df858ffdb8aa787fe94a6136e4f3583cb5c189`)

## Progress log

- 2026-08-16: User authorized PRO-09. Reviewer prepared ExecPlan and standalone DEV prompt.
- 2026-08-16: Implemented backend typed parameter discovery, AST validation without eval, train/test split, robustness scoring, ranking exclusion for < 5 trades, and cooperative sweep cancellation manager (`StrategyLabService`, `test_strategy_lab.py`).
- 2026-08-16: Implemented frontend typed parameter dropdowns, In-Sample/OOS split inputs with overlap validation, max variants bounds, cancellation button, and robustness ranking indicators (`StrategyLabPage.tsx`, `strategyLabApi.ts`, `StrategyLabPage.test.tsx`).
- 2026-08-16: Added deterministic Product UAT assertions (`pro09.*`) in `product-uat-v3-baseline.json` and `product-uat.mjs`.
- 2026-08-16: Verified fast gate (`verify-v2.ps1`) and deterministic product UAT (`run-product-uat.ps1`: 333/333 passed). Verified `backend/sumi.db` SHA-256 unchanged.
- 2026-08-16: Implementation completed. Ready for Independent Reviewer evaluation.
- 2026-08-16: Independent Reviewer audited implementation, contracts, and test evidence. Verdict: `APPROVE` recorded in `docs/reviews/PRO_09_REVIEW_2026-08-16.md`. PRO-09 is closed; PRO-10 remains unauthorized.
