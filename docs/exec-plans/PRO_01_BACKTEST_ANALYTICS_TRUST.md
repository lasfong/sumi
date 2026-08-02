# PRO-01 — Backtest and Analytics Trust

## Outcome

Backtest, Analytics, Strategy Lab, and Scanner communicate the quantitative support available for every run. Statistical metrics are nullable and explain whether they are valid, insufficient, or not applicable; small samples cannot be styled, ranked, or recommended as positive evidence. Every backtest also records coverage, execution assumptions, and a reproducibility manifest.

## Context and problem

This batch implements `PRO-BT-01` through `PRO-BT-10` from `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` after independent approval of PRO-00 at local base commit `24468dd`. Existing analytics currently synthesize zero or substitute a standard deviation for unavailable Sharpe, Sortino, and SQN values, and the UI/ranking surfaces treat raw nullable numbers inconsistently. The batch preserves all accepted V3 and PRO-00 regressions, especially `PRO-INT-01` through `PRO-INT-10`.

## In scope

- Add typed backend contracts for data coverage, execution assumptions, metric validity, and reproducibility manifests.
- Calculate honest metric results for win rate, profit factor, Sharpe, Sortino, and SQN, including documented observation thresholds and reasons.
- Report requested/actual coverage, candle/symbol/gap/exclusion information, execution timing/price/fee/tax/slippage/liquidity/sizing/settlement assumptions, and run identity on backtest results.
- Propagate metric validity through Analytics, Backtest, Strategy Lab sweep ordering, and Scanner result ordering/visibility where quantitative ranking is exposed.
- Update frontend types and user-visible Backtest, Analytics, Strategy Lab, and Scanner surfaces to show sample sizes and neutral unavailable states.
- Add deterministic hand-calculated backend fixtures, focused frontend tests, and browser UAT assertions/screenshots for insufficient and valid cases.

## Out of scope

- PRO-02 dashboard/session-picker/localization work and every later Professional batch.
- New strategy execution features, arbitrary code execution, data providers, dependencies, migrations, or acceptance-criteria changes.
- Reopening PRO-00 unless a regression is reproduced.
- Commit, push, tag, release, or modification of `backend/sumi.db`.

## Invariants

- Replay and derived responses never expose information past the server-authorized boundary.
- Backend services remain authoritative for accounting, metric calculation, validity, execution assumptions, and run manifests; FastAPI routes only validate/compose transport.
- Declarative strategy evaluation remains shared and does not use `eval` or arbitrary code.
- Invalid metrics remain `null`, carry `valid`, `insufficient_data`, or `not_applicable`, include sample size and a human-readable reason when unavailable, and cannot win a ranking surface.
- SQN requires at least 30 closed trades. Sharpe and Sortino require at least 30 daily periodic returns; Sortino additionally requires at least two downside observations.
- Tests and browser UAT use temporary databases. Production `backend/sumi.db` must retain SHA-256 `4166D749119B0EBB4B9ADF418EA18442FF6E0C14AE762147CD3D0FBE20F76459` throughout this batch.
- No telemetry or external transmission of user data.

## Current architecture

- `backend/app/services/analytics_service.py` builds equity/benchmark curves and raw analytics values. It currently returns synthetic zero for insufficient Sharpe and substitutes `1.0` standard deviations for SQN/Sortino.
- `backend/app/schemas/analytics_schema.py` exposes flat nullable scalar metrics without validity metadata.
- `backend/app/services/backtest_service.py` owns declarative single/multi-symbol runs but returns only partial candle/date summaries and no typed assumption or reproducibility contract.
- `backend/app/services/strategy_lab_service.py` extracts raw scalars and sorts every variant by raw net PnL.
- `backend/app/services/scanner_service.py` returns chronological signal rows and currently has no quantitative metric ranking contract.
- `frontend/src/pages/BacktestPage.tsx`, `AnalyticsPage.tsx`, `StrategyLabPage.tsx`, and `ScannerPage.tsx` render raw metrics and positive colors without a shared validity presentation.

## Target design

- A backend analytics-contract module owns `DataCoverage`, `ExecutionAssumptions`, `MetricResult`, and `RunManifest` schemas plus documented validity semantics.
- `AnalyticsService` computes periodic daily equity returns and trade metrics once, returning legacy scalar values only where compatibility requires them and an authoritative typed metric map for all trust-sensitive presentation/ranking.
- `BacktestService` derives actual coverage from queried candles, explicitly records gaps/exclusions and all modeled/unmodeled assumptions, and hashes stable strategy/data inputs into a reproducibility manifest. Volatile run timestamp is recorded but excluded from deterministic input identity.
- Strategy Lab carries typed metric results and applies a validity-aware stable ordering. Scanner carries explicit ranking validity; when no supported quantitative score exists, rows remain unranked rather than implying a best result.
- Frontend helpers render a metric value only when `status === "valid"`; otherwise they render a neutral unavailable state, sample size, and reason. Win rate and profit factor always include closed-trade count.
- Existing storage remains readable without migration; new contracts are response-only in this batch.

## Milestones

1. Contract and deterministic metric core: typed schemas and hand-calculated fixtures pass for zero/one/30-trade, all-win/all-loss, flat-equity, missing benchmark, partial coverage, fees/taxes/PnL/drawdown, and valid statistical cases.
2. Backtest and consumer propagation: coverage, assumptions, manifests, Strategy Lab validity-aware ordering, and Scanner unranked/validity behavior pass focused service/API tests.
3. User-facing trust presentation: all four product surfaces show sample/reason/neutral styling and focused frontend/browser assertions pass at 1440×1000 and 1280×800.
4. Regression and evidence closure: full backend/frontend/lint/build, `verify-v2`, standalone product UAT, and `verify-product` pass with retained machine-readable evidence, screenshots, hashes, and cleanup diagnostics.

## Acceptance mapping

| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| PRO-BT-01 | Backtest `data_coverage` contract | Requested/actual range, symbol/candle/gap/exclusion fixtures and browser details |
| PRO-BT-02 | Backtest `execution_assumptions` contract | Exact contract/API fixture and browser details |
| PRO-BT-03 | Typed `MetricResult` map | Zero/small/valid sample fixtures and UI assertions |
| PRO-BT-04 | SQN threshold and no fake deviation | 29/30-trade and flat/all-equal fixtures |
| PRO-BT-05 | Daily-return Sharpe/Sortino thresholds | 29/30 returns, downside-count, and hand-calculated fixtures |
| PRO-BT-06 | Win-rate/profit-factor sample metadata | All-win/all-loss/small-sample service and UI fixtures |
| PRO-BT-07 | Honest edge-case statuses | Zero/one/all-win/all-loss/flat/missing-benchmark/partial-coverage fixtures |
| PRO-BT-08 | Deterministic ledger verification | PnL, fees, taxes, drawdown, benchmark, and statistical fixtures |
| PRO-BT-09 | Shared validity-aware consumer/ranking contract | Strategy Lab ordering and Scanner/Backtest/Analytics UI tests |
| PRO-BT-10 | `run_manifest` contract | Stable strategy/data/assumption identity and timestamp fixture |
| PRO-INT-01–10 | No changes to authorization boundaries | Focused PRO-00 regression plus full product UAT |

## Verification commands

```powershell
git status --short
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
cd backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_analytics_trust.py app/tests/test_backtest_trust.py app/tests/test_strategy_lab.py app/tests/test_scanner.py -q
& .\.venv\Scripts\python.exe -m pytest app/tests/test_scanner.py app/tests/test_replay_no_future_leak.py app/tests/test_ws_replay.py app/tests/test_scanner_replay_integrity.py -q
& .\.venv\Scripts\python.exe -m pytest -q
cd ..\frontend
npm.cmd test -- --run src/pages/__tests__/BacktestPage.test.tsx src/pages/__tests__/AnalyticsPage.test.tsx src/pages/__tests__/StrategyLabPage.test.tsx src/pages/__tests__/ScannerPage.test.tsx
npm.cmd test -- --run
npm.cmd run lint
npm.cmd run build
cd ..
& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-v2.sh"
powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1
& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-product.sh"
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

## Rollback and compatibility

The batch adds response metadata and frontend presentation without a database migration. Existing stored sessions, trades, scanner runs, and Strategy Lab history remain readable. Rollback is a bounded revert of the PRO-01 diff; no stored-data restoration is required. If a compatibility conflict requires a destructive or contract migration, stop for Reviewer direction.

## Risks and mitigations

- Existing tests/clients may rely on flat scalars: preserve compatible scalar fields where honest while making typed metrics authoritative and update all in-repository consumers together.
- Daily periodic-return semantics may be confused with per-trade returns: label period/frequency explicitly and cover known-equity fixtures.
- Infinite profit factor for all-win ledgers is not JSON-safe evidence: represent it as unavailable/not-applicable with a reason instead of infinity.
- Multi-symbol aggregation can imply unsupported statistics: aggregate only metrics with mathematically valid pooled observations, otherwise mark unavailable with a reason.
- Ranking can silently coerce null to zero: use explicit validity tuples and deterministic ties, with tests proving invalid results cannot lead.
- UAT must not grow a fail-open assertion path: add blocking manifest entries and negative reconciliation tests before relying on new browser assertions.

## Progress log

- 2026-08-02: Read `AGENTS.md`, the full PRO-01 prompt, canonical V3 sources, Professionalization Master Plan, PRO-00 ExecPlan, and `PLANS.md`. Confirmed local `master` at clean PRO-00-approved base `24468dd` except the pre-existing untracked PRO-01 prompt; recorded production DB SHA-256 `4166D749119B0EBB4B9ADF418EA18442FF6E0C14AE762147CD3D0FBE20F76459`.
- 2026-08-02: Created this ExecPlan before product code. Milestone 1 is in progress.
- 2026-08-02: Added typed `MetricResult`, `DataCoverage`, `ExecutionAssumptions`, and `RunManifest` schemas; backend analytics now enforce 30-trade SQN/win-rate/profit-factor evidence thresholds, 30 daily-return Sharpe/Sortino thresholds, two-downside-observation Sortino threshold, and zero-variance/unavailable reasons.
- 2026-08-02: Propagated validity through Backtest/Analytics presentation, Strategy Lab ranking eligibility, and explicitly chronological-unranked Scanner signals. Added additive blocking PRO-01 manifest assertions while preserving the sealed 265-ID V3 baseline and all 10 PRO-00 assertions.
- 2026-08-02: Focused verification passed: 23 backend tests and 7 frontend tests; production build passed. Full standalone technical verification passed: backend 112 passed/1 skipped, frontend 133 passed, lint/build and `verify-v2` passed.
- 2026-08-02: First complete product UAT retained at `test-results/product-uat/2026-08-02T05-47-51-354Z/results.json` and correctly failed. It reproduced an in-scope consistency defect: the immediate Backtest response used 369 daily returns, while reopened Analytics used the persisted last-fill boundary of 72 returns. The same run also duplicated one Scanner assertion because the helper is called for both blind/review creation.
- 2026-08-02: Corrected the defect by committing/refreshing the final analyzed backtest session boundary before analytics and added a reopened-session regression assertion. Guarded the Scanner check to execute once. The justified standalone rerun passed 286/286.
- 2026-08-02: Final `verify-product.sh` passed after synchronizing the Strategy Lab history-save response so no API write is aborted by UAT navigation. Milestones 1–4 are complete; DEV is stopped at independent Reviewer gate.
- 2026-08-02: Independent Reviewer returned bounded REWORK findings: include initial cash/benchmark in manifest identity; honor configured benchmark; derive MACD warm-up from actual finite output; restore immutable PRO-00 manifest sealing; and add all-loss, missing-benchmark, and partial-coverage/gap fixtures. No scope expansion is authorized.
- 2026-08-02: Resolved all Reviewer findings in scope. `RunManifest.input_hash` now includes initial cash and benchmark symbol; backtest sessions persist benchmark identity and Analytics honors it on reopen; warm-up is derived from first finite StrategyIndicatorAdapter outputs (MACD 33); PRO-00’s sealed 10-ID hash/count is enforced independently of later additive batches; deterministic edge-case fixtures cover all-loss, missing benchmark, and partial gaps/warm-up.
- 2026-08-02: Rework focused verification passed: 27 backend tests in the focused PRO-01 suite, 7 manifest/self-check tests, and the explicit MACD warm-up/configured-benchmark regressions. Final full product gate passed with backend 115 passed/1 skipped, frontend 133 passed, lint/build/verify-v2 and UAT green; post-gate full backend rerun passed 116/1 skipped after the additional configured-benchmark fixture.
- 2026-08-02: Reviewer re-review found one remaining P1: AnalyticsPage hard-coded VNINDEX and styled benchmark return from curve data instead of the typed validity contract. Added `benchmark_symbol` to the typed backend response, authoritative MetricResultValue rendering, alternate/missing benchmark frontend tests, and one additive browser assertion. Final full gate passed with backend 116 passed/1 skipped and frontend 135 passed.

## Decision log

- Use additive response contracts without persistence migration. This preserves stored sessions/history and keeps rollback bounded.
- Treat daily equity-curve changes as the documented periodic-return basis for Sharpe and Sortino. Trade PnL samples remain the basis for SQN.
- Do not invent a Scanner quality score. Scanner signals are explicitly unranked unless a supported valid metric is supplied by the shared contract.
- Preserve legacy flat descriptive scalars where stored-history/API compatibility requires them, but make the typed metric map authoritative for evidence, ranking, and UI. Trust-sensitive legacy Sharpe, Sortino, and SQN fields are `null` when unavailable.
- Treat total net PnL as a valid arithmetic ranking input only when at least one closed trade exists; failed/zero-trade variants are not ranking-eligible. Win rate, profit factor, expectancy, and SQN remain unavailable below their evidence threshold.
- Persist the final analyzed backtest boundary. A trade lifecycle commit may precede the end of a run, so relying on the last fill commit makes later Analytics irreproducible.
- Hash ordered candle timestamps, OHLCV values, timeframe, and adjustment semantics into `data_identity`; range/count alone cannot identify corrected data reproducibly.

## Deviations log

- The original verification command in this plan referenced `backend/.venv` relative to `backend/`; on this Windows checkout the executable required an absolute path plus `PYTHONPATH=E:\Workspace\sumi\backend`. The canonical Git Bash wrappers used `SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe` successfully.
- The first full UAT was not blindly repeated. Its retained 286-check result identified the final-boundary persistence defect and duplicate helper assertion; a focused regression passed before the justified rerun.
- `frontend` types accept missing metric maps for read compatibility with older locally saved Strategy Lab history. The live backend contract always emits the typed map; the UI renders missing historical validity as neutral unavailable, never as positive evidence.

## Completion evidence

Implementation and DEV verification are complete. Independent Reviewer inspection is approved as recorded below.

- Base: local `master` commit `24468ddbb01e4fa790dcf1c8ea67cd3ef816d9db`; the pre-existing untracked `docs/dev-prompts/PRO_01_BACKTEST_ANALYTICS_TRUST_PROMPT.md` was preserved.
- Final focused backend command: `$env:PYTHONPATH='E:\Workspace\sumi\backend'; & 'E:\Workspace\sumi\backend\.venv\Scripts\python.exe' -m pytest app/tests/test_analytics.py app/tests/test_analytics_advanced.py app/tests/test_analytics_known_ledger.py app/tests/test_analytics_trust.py app/tests/test_backtest.py app/tests/test_strategy_lab.py app/tests/test_scanner.py -q` — 27 passed, one existing Starlette/httpx deprecation warning.
- Final focused frontend command: `npm.cmd test -- --run src/pages/__tests__/AnalyticsPage.test.tsx` — 3 passed, covering alternate configured benchmark identity and missing-benchmark neutral reason/style. The earlier four-surface focused suite passed 7 tests before these two additive cases.
- Manifest self-test: `node --test scripts/product-uat-manifest.test.mjs` — 7 passed. `node --check scripts/product-uat.mjs` passed.
- Final full gate backend: 116 passed, 1 skipped, one existing deprecation warning. Final full frontend: 135 passed across 22 files. Frontend lint and production build passed.
- `& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-v2.sh"` — passed all six steps; browser smoke is intentionally delegated to the product gate.
- `powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1` — corrected standalone path was exercised; the final canonical product gate reran the same isolated UAT after Reviewer rework.
- `& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-product.sh"` — final exit 0; backend 116 passed/1 skipped, frontend 135, lint/build/verify-v2 and product UAT passed.
- Final machine-readable evidence: `test-results/product-uat/2026-08-02T06-29-03-858Z/results.json`; sealed baseline 265, declared/actual/passed 288/288/288; zero failed, blocking failed, missing, unexpected, duplicate, blocking-mismatch, runtime, provider, or failed blocking IDs. Manifest SHA-256 `164fcbbced79d8aac603710465365f6282b0de8c2982b76eac7a0e73faa29983`.
- The final run recorded 73 non-OK observations: 71 expected Vite `304` cache responses plus the assertion-covered expected workflow `400` and `409`. Five request failures were Vite asset `net::ERR_ABORTED` cancellations during deliberate route transitions; no API request failed after Strategy Lab save synchronization.
- Final PRO-01 screenshots were visually reviewed. `pro01-backtest-trust-1440x1000.png` is 1440×1000, SHA-256 `2778dc96477026cb689ab03611f850c8cf92352fa92da36201b3831426ab1fd5`; it visibly shows one closed trade, unavailable win rate with threshold reason, and 369-return Sharpe evidence. `pro01-analytics-trust-1280x800.png` is 1280×800, SHA-256 `0a06c162603dc0f2a77afa2625911401400aab24dbd06bf1db5f9d91d6208c4f`; it visibly labels `VNINDEX benchmark`, shows the authoritative 370-candle sample, and uses neutral MetricResultValue semantics.
- Temporary DB `C:/Users/hieup/AppData/Local/Temp/sumi-uat-f4b2eb7b/sumi-uat.db` was removed. Zero listeners remain on ports 18000/15173 and no owned Uvicorn/Vite/Playwright/Chromium process remains.
- Production DB before/after/current SHA-256 is exactly `4166D749119B0EBB4B9ADF418EA18442FF6E0C14AE762147CD3D0FBE20F76459`.
- DEV self-review found only PRO-01 contract/service/test/frontend/UAT/ExecPlan changes plus the preserved prompt. No dependency, migration, provider, external transmission, PRO-02 work, commit, push, or production DB mutation occurred.
- No accepted V3, PRO-00, or product-UAT assertion was removed, renamed, weakened, or made non-blocking. One legacy unit expectation that treated four-trade Sharpe/Sortino/SQN as available was replaced by the stricter immutable PRO-BT threshold contract and explicit sample/reason assertions; this strengthens rather than weakens acceptance coverage.
- Reviewer should inspect typed/legacy compatibility, hand-calculated threshold fixtures, final-boundary persistence, Strategy Lab ranking eligibility, Scanner unranked semantics, 12 additive manifest assertions, both screenshots, DB hashes, and the retained first-failure diagnostic before approving PRO-01. Do not start PRO-02 until approval.
- Reviewer rework checklist is now satisfied: configured benchmark survives response/reopen; manifest identity changes with cash/benchmark and hashes candle content; MACD finite output warm-up is covered by a focused test; deleting/renaming a PRO-00 assertion fails closed; all-loss/missing-benchmark/partial-gap fixtures assert honest values/reasons; one additive benchmark UAT assertion is blocking and reconciled.

Independent Reviewer approval — 2026-08-02: APPROVED. Reviewer independently reran focused backend (27 passed), frontend trust surfaces (9 passed), and manifest fail-closed tests (7 passed); inspected the complete diff, final 288/288 UAT reconciliation, both required screenshots, benchmark identity/alternate/missing states, production DB hash, and cleanup. No remaining P0/P1/P2 finding, scope expansion, weakened accepted assertion, dependency, migration, provider change, or PRO-02 work was found. PRO-02 is unlocked by acceptance order, but no commit or push was performed.
