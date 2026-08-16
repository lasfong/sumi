# Sumi machine-transfer handoff — 2026-08-10

This is the single entry point when Sumi is moved to another computer. It records what is authoritative, what must be copied, what is already approved, and the exact instruction that starts the next bounded batch. Chat history is not required.

## 1. Transfer checkpoint

- Repository: `https://github.com/lasfong/sumi.git`
- Branch: `master`
- Pushed remote: `origin/master` contains PRO-00, PRO-01, PRO-02, PRO-03, PRO-04, PRO-05, PRO-06, PRO-07, PRO-08, and PRO-09 commits.
- Approved program state: PRO-00 through PRO-10 approved. PRO-11 through PRO-12 not started.
- Latest verdict: `docs/reviews/PRO_10_REVIEW_2026-08-16.md` — `APPROVE`.
- Authoritative reviewer UAT: `test-results/product-uat/2026-08-16T15-02-02-832Z/results.json` — 333/333 passed, 0 failed, 0 blocking failed, no runtime errors.
- Production database: `backend/sumi.db`, 620,945,408 bytes, SHA-256 `450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A`.
- Reviewer results SHA-256: `8937BF06D7938A5FC7D197E6293C1E81DEF001E371191BE930BE5181B6FBE07E`.
- Workspace state: PRO-00 through PRO-09 committed to `master`; PRO-10 implementation and reviewer approval documents are in the working tree.

PRO-10 is approved and closed. PRO-11 implementation has not started.

## 2. What must move to the new computer

### Versioned workspace layer — mandatory

Clone or pull `master` from `https://github.com/lasfong/sumi.git`. Because PRO-03 is committed and pushed to `origin/master`, `git clone` or `git pull` supplies the entire versioned code, tests, migrations, dossiers, and handoff documentation.

Do not copy machine-specific dependency caches. Recreate these on the destination:

- `backend/.venv/`
- `frontend/node_modules/`
- root or spike `node_modules/`
- `dist/`, `.vite/`, `.pytest_cache/`, and `__pycache__/`

### Local-first data layer — mandatory when preserving the current workstation data

These paths are ignored by Git and require a separate copy:

- `backend/sumi.db`
- `data/raw/` and other user-owned files under `data/` (currently about 592 MB total)
- any local backups or exports the user needs

After copying `backend/sumi.db`, verify its SHA-256 before opening the application or running migrations.

### Reviewer evidence layer — mandatory for durable audit continuity

Copy at least this complete directory:

`test-results/product-uat/2026-08-12T13-58-09-705Z/`

It contains `results.json` and the retained screenshots. The directory is ignored by Git. Other historical `test-results/` directories are optional unless the user wants the full audit history.

## 3. Recommended transfer method

1. **Code & Docs**: Perform a standard `git clone https://github.com/lasfong/sumi.git` or `git pull origin master` on the new machine.
2. **Local Data & Test Evidence**: Copy `backend/sumi.db`, `data/raw/`, and `test-results/product-uat/2026-08-12T13-58-09-705Z/` via local network, external drive, or secure file transfer.

Before disconnecting the old machine, retain two independent copies of `backend/sumi.db`. Do not delete or overwrite the old workspace until the destination passes the checks below.

## 4. Destination bootstrap

Required toolchain:

- Git
- Python 3.12 or newer (`pandas-ta==0.4.71b0` requires it)
- Node.js compatible with the checked-in frontend lockfile; the source machine used Node `v24.14.0` and npm `11.9.0`
- PowerShell on Windows for the repository-supported gate wrappers

From the destination repository root:

```powershell
py -3.12 -m venv backend\.venv
& backend\.venv\Scripts\python.exe -m pip install --upgrade pip
& backend\.venv\Scripts\python.exe -m pip install -r backend\requirements.txt
Set-Location frontend
npm.cmd install
Set-Location ..
```

Do not seed or import into `backend/sumi.db` during handoff verification.

## 5. Destination integrity checks

Run these before any implementation:

```powershell
git branch --show-current
git rev-parse HEAD
git status --short
Get-FileHash -Algorithm SHA256 backend\sumi.db
Get-FileHash -Algorithm SHA256 test-results\product-uat\2026-08-12T13-58-09-705Z\results.json
git diff --check
```

Expected branch/commit and both expected hashes are recorded in section 1. The dirty-file inventory must be preserved; a clean destination at commit `bc82434` means the uncommitted PRO-03 workspace was lost.

After dependencies are installed, run the isolated fast gate:

```powershell
.\scripts\verify-v2.ps1
```

Record the database hash before and after; both must equal the value in section 1. Product UAT need not be rerun merely to copy the workspace if the retained reviewer artifact and hashes match. Rerun it when a later PRO reaches its Reviewer gate.

## 6. Canonical read order on the new computer

1. `AGENTS.md`
2. this file
3. `docs/INDEX.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
6. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
7. `docs/SESSION_HANDOFF_PROTOCOL.md`
8. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
9. `docs/reviews/PRO_04_REVIEW_2026-08-15_R5.md`
10. `docs/reviews/PRO_05_REVIEW_2026-08-15.md`
11. `docs/reviews/PRO_06_REVIEW_2026-08-16.md`
12. `docs/reviews/PRO_07_REVIEW_2026-08-16.md`
13. `docs/reviews/PRO_08_REVIEW_2026-08-16.md`
14. `docs/reviews/PRO_09_REVIEW_2026-08-16.md`
15. `docs/reviews/PRO_10_REVIEW_2026-08-16.md`
16. `docs/exec-plans/PRO_10_DATA_PROVIDER_EVALUATION.md`

Old V2 documents, historical V3 Batch 0–5 records, `docs/tester/`, and superseded handoffs are evidence only. They do not override the files above.

## 7. Program outcome in one view

| Batch | Status | User outcome |
| --- | --- | --- |
| PRO-00 | Approved | Honest blind/signal-review replay and fail-closed UAT authority. |
| PRO-01 | Approved | Backtest and analytics metrics report validity and refuse false precision. |
| PRO-02 | Approved | Dashboard, Replay, Journal, and Analytics operate as one daily workflow. |
| PRO-03 | Approved | Catalog/import capability with stale-preview acceptance fail-closed. |
| PRO-04 | Approved | Release SMA, Bollinger Bands, ATR, and Volume SMA with exact parameter/output contracts. |
| PRO-05 | Approved | Release MFI, Stochastic, ADX, and Relative Strength vs VNINDEX. |
| PRO-06 | Approved | Release Keltner Channels, PSAR, and SuperTrend. |
| PRO-07 | Approved | Release Ichimoku with an explicit no-look-ahead displacement contract. |
| PRO-08 | Approved | Add risk-based trade planning, position sizing, checklist, and richer journal review. |
| PRO-09 | Approved | Make strategy comparison reproducible and resistant to overfitting. |
| PRO-10 | Approved | Approve market-data provider under Provider Boundary Adapter architecture. |
| PRO-11 | Conditional | Add explicit one-click local data synchronization with preview/rollback under `MarketDataProviderAdapter`. |
| PRO-12 | Not started | Produce the independently verified Professional release candidate. |

After PRO-12, the intended result is a dependable local-first Vietnam-market replay, technical-analysis, data-management, trading-practice, journaling, and strategy-research workstation with evidence-backed release quality.

## 8. Exact next action

PRO-10 is closed and independently approved. PRO-11 remains unauthorized until explicit user instruction. No further session actions are authorized without user prompt.

## Appendix A — historical transferred worktree inventory

This appendix records the original 2026-08-10 transfer snapshot only. It is historical and does not describe the current dirty-worktree inventory; section 8 and the state ledger are the current authority.

- `docs/AUTONOMOUS_EXECUTION_STATE.md`
- `docs/INDEX.md`
- `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md`
- `docs/dev-prompts/PRO_03_STALE_PREVIEW_REWORK_PROMPT.md`
- `docs/reviews/PRO_03_REVIEW_2026-08-10_R3.md`

The source checkpoint has 23 modified entries and 21 untracked entries. `docs/program/` is shown by Git as one untracked directory but contains the PRO-03 through PRO-12 dossiers listed in `docs/program/README.md`.

```text
 M README.md
 M backend/app/api/import_data.py
 M backend/app/api/symbols.py
 M backend/app/models/__init__.py
 M backend/app/schemas/import_schema.py
 M backend/app/services/cafef_importer.py
 M backend/app/tests/conftest.py
 M backend/app/tests/test_cafef_importer.py
 M backend/scripts/import_batch.py
 M backend/seed_cafef.py
 M docs/AUTONOMOUS_EXECUTION_STATE.md
 M docs/INDEX.md
 M docs/PROFESSIONALIZATION_HANDOFF_2026-08-01.md
 M docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md
 M docs/exec-plans/PRO_02_DAILY_TRADER_WORKFLOW.md
 M frontend/src/api/importApi.ts
 M frontend/src/hooks/__tests__/useSessionSelection.test.tsx
 M frontend/src/hooks/useSessionSelection.ts
 M frontend/src/pages/ImportPage.tsx
 M frontend/src/pages/__tests__/DashboardPage.test.tsx
 M scripts/fixtures/product-uat-v3-baseline.json
 M scripts/product-uat.mjs
 M scripts/verify-v2.ps1
?? backend/app/services/import_classifier.py
?? backend/app/services/import_workflow_service.py
?? backend/app/services/weekly_aggregator.py
?? backend/app/tests/test_import_api.py
?? backend/app/tests/test_import_classifier.py
?? backend/app/tests/test_import_workflow.py
?? backend/app/tests/test_weekly_aggregator.py
?? docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md
?? docs/SESSION_HANDOFF_PROTOCOL.md
?? docs/dev-prompts/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY_LOW_MODEL_PROMPT.md
?? docs/dev-prompts/PRO_04_CORE_INDICATOR_EXPANSION_LOW_MODEL_PROMPT.md
?? docs/exec-plans/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md
?? docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md
?? docs/program/
?? docs/reviews/PRO_02_REVIEW_2026-08-09.md
?? docs/reviews/PRO_03_REVIEW_2026-08-09.md
?? docs/reviews/PRO_03_REVIEW_2026-08-10.md
?? docs/reviews/PRO_03_REVIEW_2026-08-10_R2.md
?? docs/tester/README.md
?? scripts/negative-operation-tracker.mjs
?? scripts/negative-operation-tracker.test.mjs
```
