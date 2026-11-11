# SUMI V3 — VERIFIED PROJECT SNAPSHOT

- **Snapshot ID**: `SUMI-V3-SNAPSHOT-2026-10-20-V7`
- **Verification Timestamp**: `2026-10-20T16:10:03+07:00`
- **Verified By Session**: `SUMI-REL001-BASELINE-CLOSE-001`
- **Repository Root**: `/Users/mizuhara/workspace/sumi`

> [!IMPORTANT]
> **Phase A Reconciliation Rule**: This snapshot provides the concise current-state orientation for **Phase A (Minimal Bootstrap)** of [`SESSION_PROTOCOL.md`](SESSION_PROTOCOL.md). Every new session must still execute Phase A Step 5 (`git rev-parse HEAD`, `git status --short --untracked-files=all`, `shasum -a 256 backend/sumi.db`) to verify that live workspace state matches this snapshot before reading Phase B documents or taking any action.

---

## 1. Git & Working-Tree State (`VERIFIED CURRENT STATE`)

- **Active Branch**: `master` (`ahead 10` of `origin/master`)
- **Current `HEAD` Commit**: `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab` (`864e751`)
- **Latest Controlled Remediation Commit Details**:
  - **SHA**: `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`
  - **Parent SHA**: `d2d048a507370a787a750984e3c1a89690078613`
  - **Date**: `2026-10-20 16:10:03 +0700`
  - **Subject**: `fix(v3): preserve valid CafeF candles with audited OHLC normalization` (`9` files: `5` backend production files, `2` frontend production files, `2` regression test files; `1,207` insertions, `54` deletions)
- **Recent Commit Lineage on `master` (`VERIFIED CURRENT STATE`)**:
  1. `864e751` (`2026-10-20`): `fix(v3): preserve valid CafeF candles with audited OHLC normalization`
  2. `d2d048a` (`2026-10-19`): `fix(v3): resolve full-browser QA acceptance defects`
  3. `925a095` (`2026-10-17`): `fix(v3): resolve independent QA financial and product acceptance defects`
  4. `bf0bbc6` (`2026-10-17`): `fix(v3): complete W5 product UAT remediation and governance records`
  5. `40f6563` (`2026-10-16`): `feat(v3): complete W4 API contracts trading practice and strategy UX`
  6. `12e166a` (`2026-10-16`): `feat(v3): complete W3 replay state websocket and chart integration`
  7. `0732ca0` (`2026-10-16`): `docs(governance): establish Sumi V3 execution charter`
  8. `c331dc3` (`2026-10-16`): `feat(v3): complete W0-W2 execution and financial correctness baseline`
  9. `2151f34` (`2026-10-14`): `docs(plan): add V3 architecture baseline, test strategy, and reconciled product completion plan`
  10. `e34516c` (`2026-10-14`): `chore(stabilization): W0-A integrated stabilization for B00, B01, B02 with fail-closed DB safeguard`
- **Working-Tree Dirty State (`VERIFIED CURRENT STATE`)**:
  1. **Preserved Uncommitted Tracked Governance Modifications (`3` files — `3 files changed, 56 insertions(+), 9 deletions(-)`)**:
     - `M docs/plans/product-completion/DECISION_LOG.md` (`+15 -0`)
     - `M docs/plans/product-completion/EXECUTION_STATE.md` (`+31 -4`)
     - `M docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md` (`+10 -5`)
     - **Protection & Authority Rule**: These three files were modified on `2026-10-17` after commit `925a095` and **lag behind `HEAD` (`864e751`, `2026-10-20`) by two commits (`d2d048a` and `864e751`)**. Do **not** edit, reset, stage, or commit these three files without explicit Product Owner authorization.
  2. **Application & Test Source Working Tree (`0` uncommitted changes)**:
     - All `9` `REL-001` source and test files (`7` modified + `2` new test files) are now cleanly committed on `master` at `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`.
  3. **Untracked Managed Session-Context Files (`6` files)**:
     - `?? docs/operations/agent-sessions/CONTEXT_INDEX.md`
     - `?? docs/operations/agent-sessions/HANDOFF_TEMPLATE.md`
     - `?? docs/operations/agent-sessions/NEXT_TASK.md`
     - `?? docs/operations/agent-sessions/PROJECT_SNAPSHOT.md`
     - `?? docs/operations/agent-sessions/SESSION_PROTOCOL.md`
     - `?? docs/operations/agent-sessions/TASK_REGISTRY.md`
  4. **Git-Ignored Local Session & QA Evidence (`artifacts/local/`)**:
     - `artifacts/local/session-ops/CTX-BOOTSTRAP-001/` (initial bootstrap handoff & logs)
     - `artifacts/local/session-ops/CTX-HARDEN-002/` (protocol-hardening handoff, dry-run report & logs)
     - `artifacts/local/session-ops/OWNER-UAT-PREP-001/` (preserved Owner UAT preparation handoff, `OWNER_UAT_RUNBOOK.md`, `OWNER_UAT_SCORECARD.md`, `test_db/sumi_owner_uat_isolated.db`, `test_db/sumi_owner_uat_baseline_snapshot.db`, `test_db/FIXTURE_MANIFEST.json`, `test_db/fixtures/*.csv`, and `logs/*`)
     - `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/` (independent real-world release readiness assessment: `reports/{FINAL_RELEASE_READINESS_REPORT,RELEASE_FEATURE_COVERAGE_MATRIX,PRODUCT_QUALITY_SCORECARD,RELEASE_ISSUE_REGISTER,END_TO_END_JOURNEY_RESULTS,OPERATIONAL_READINESS}.md`, `evidence/RELEASE_GATE_RESULTS.json`, `screenshots/*` (`65` PNGs), `test_db/sumi_release_gate_isolated.db`, `SESSION_HANDOFF.md`, `SESSION_RESULT.json`)
     - `artifacts/local/session-ops/SUMI-REL001-REMEDIATION-001/` (bounded `REL-001` CafeF raw data import correctness remediation: `REL001_ROOT_CAUSE_REPORT.md`, `REL001_DATA_RECONCILIATION.md`, `REL001_CORRECTION_POLICY.md`, `REL001_TEST_REPORT.md`, `REL001_BROWSER_VERIFICATION.md`, `SESSION_HANDOFF.md`, `SESSION_RESULT.json`, `scripts/*`, `data_samples/*`, `screenshots/*` (`9` PNGs), `test_db/*`, `logs/*`)
     - `artifacts/local/session-ops/SUMI-REL001-INDEPENDENT-RETEST-001/` (independent data integrity & release acceptance QA retest of `REL-001`: `INDEPENDENT_REL001_RETEST_REPORT.md`, `RAW_DATA_RECONCILIATION.md`, `NORMALIZATION_POLICY_CHALLENGE.md`, `BROWSER_ACCEPTANCE_RESULTS.md`, `DOWNSTREAM_PARITY_CHECK.md`, `REGRESSION_RESULTS.md`, `ISSUE_REGISTER.md`, `SESSION_HANDOFF.md`, `SESSION_RESULT.json`, `evidence/*`, `screenshots/*` (`11` PNGs), `test_db/*`, `logs/*`)
     - `artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/` (controlled commit & baseline closure of `REL-001` at `864e751`: `REL001_BASELINE_CLOSURE_REPORT.md`, `COMMITTED_FILE_INVENTORY.md`, `SESSION_HANDOFF.md`, `SESSION_RESULT.json`, `logs/*`)
     - `artifacts/local/full-browser-qa-acceptance-gate/` (commit `d2d048a` baseline handoff, gate report, logs, screenshots, and seeded `test_db/sumi_acceptance_gate_isolated.db`)

---

## 2. Product Readiness & Owner UAT Semantics

| Dimension | Current Status | Epistemic Classification | Semantic Rule |
| :--- | :--- | :--- | :--- |
| **Source Implementation (`B00–B51` + QA Remediations + `REL-001`)** | `COMMITTED_ON_MASTER` (`864e751`) | `VERIFIED CURRENT STATE` | Cleanly committed on `master` at `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab` (parent `d2d048a507370a787a750984e3c1a89690078613`). Zero uncommitted application or test changes remain. |
| **Automated Technical & Browser QA Gate (`QA-BROWSER-GATE-001`)** | `COMPLETED` (`AUTOMATED_UAT_PASS`) | `HISTORICAL EVIDENCE` | Recorded in [`FINAL_ACCEPTANCE_GATE_REPORT.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_ACCEPTANCE_GATE_REPORT.md) and [`FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md). |
| **Owner UAT Preparation (`OWNER-UAT-PREP-001`)** | `COMPLETED` (`EXECUTED_AND_VERIFIED`) | `VERIFIED CURRENT STATE` | Isolated UAT DB (`sumi_owner_uat_isolated.db`), `OWNER_UAT_RUNBOOK.md`, and `OWNER_UAT_SCORECARD.md` are preserved untouched (`SHA-256: 33237401...`). |
| **Real-World Release Readiness Assessment (`SUMI-RELEASE-GATE-001`)** | `COMPLETED` (`CONDITIONAL_GO`, `9.33/10`) | `HISTORICAL EVIDENCE` | Identified `REL-001` (raw CafeF split-rounding rejection, now fixed and committed in `864e751`) and non-blocking `REL-002..006` in `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/reports/`. Release verdict is not changed to `GO` automatically without Product Owner decision. |
| **`REL-001` CafeF Raw Data Import Remediation (`SUMI-REL001-REMEDIATION-001`)** | `COMPLETED` (`COMMITTED_ON_MASTER` @ `864e751`) | `VERIFIED CURRENT STATE` | Deterministically normalizes CafeF 4-decimal kVND split-adjustment rounding artifacts (`HPG` 2024 `250/250` rows, `22` normalized, `0` rejected; HSX 2024 `14,321/14,322` normalized, `1` real tick corruption `BTP 2024-01-18` rejected), preserves raw provenance, surfaces explicit UI badges/warnings, and passes `961` BE (`962` items across all 3 test roots) + `339` FE tests + `6/6` real-browser scenarios (`1440×1000`). |
| **`REL-001` Independent Retest & Controlled Baseline Closure (`SUMI-REL001-INDEPENDENT-RETEST-001` & `SUMI-REL001-BASELINE-CLOSE-001`)** | `AWAITING_OWNER_REVIEW` (`COMMITTED_ON_MASTER` @ `864e751`) | `VERIFIED CURRENT STATE` | Independently verified (`Tests A–F`), re-verified in pre-commit gates (`961 passed, 1 skipped` BE across 3 roots, `105 passed` focused REL-001, `103 passed` Golden/Safety, `0` TS errors, `build` PASS, `51 files / 339 passed` FE Vitest), and closed as 1 controlled commit (`864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`, `9` files). |
| **Manual Product Owner UAT (`OWNER-UAT-SIGNOFF-001`)** | **`BLOCKED` (`UNEXECUTED`, `OWNER_ACCEPTED: false`)** | `PENDING OWNER DECISION` | **Manual Owner UAT has NOT been executed.** Only the Product Owner may execute manual UAT, grant `OWNER_ACCEPTED: true` (`ACCEPTED_V3_COMPLETE: true`), and decide final release disposition. |

---

## 3. Verification Evidence (Strictly Distinguished by Session & Epistemic Status)

### 3.0 Checks Executed in Current Session `SUMI-REL001-BASELINE-CLOSE-001` (`VERIFIED CURRENT STATE`)
The following checks were directly executed in this session (`artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/`):
- **Pre-Commit Source & Policy Integrity Audit**: Reconciled all `9` paths (`7` modified source files + `2` new test files) against `SUMI-REL001-REMEDIATION-001` and `SUMI-REL001-INDEPENDENT-RETEST-001` (`logs/rel001_source_diff.patch`), confirming all 5 preconditions in `CafeFImporter.evaluate_and_normalize_ohlc` are intact (`0` code drift).
- **Pre-Commit Automated Test Gates (`VERIFIED CURRENT STATE`)**:
  - **Focused REL-001 & Import Suite**: `105 passed in 4.31s` (`logs/pytest_focused_and_golden.log`).
  - **Financial Golden Cases (`G-01..G-12`) & Quarantine/Transaction Safety**: `103 passed in 2.44s` (`logs/pytest_focused_and_golden.log`).
  - **Complete Repository Backend Pytest (`backend/app/tests`, `backend/tests`, `tests`)**: `961 passed, 1 skipped in 24.18s` (`962` items collected, `logs/pytest_complete_repo.log`).
  - **Frontend TypeScript, Production Build & Full Vitest**: `npx tsc --noEmit` (`0` errors), `npm run build` (`built in 326ms`), `ImportPageRel001.test.tsx` (`1 passed`), Full Vitest (`51 test files passed, 339 tests passed`, `logs/frontend_complete.log`).
- **Controlled Git Commit & Protected DB Verification (`VERIFIED CURRENT STATE`)**:
  - Created exactly 1 commit: `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab` (parent `d2d048a507370a787a750984e3c1a89690078613`, `9` files, `1,207` insertions, `54` deletions, `logs/git_commit_verification.log`).
  - `backend/sumi.db` unchanged (`188,416` bytes, SHA-256 `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`).
  - `sumi_owner_uat_isolated.db` unchanged (`1,462,272` bytes, SHA-256 `33237401c6d20793ca96844a5fc8e13d85c6d504d28c0ad3ea7133aad5641294`).
  - Protected governance files unchanged (`3 files changed, 56 insertions(+), 9 deletions(-)`).

### 3.0b Checks Executed in Prior Sessions `SUMI-REL001-INDEPENDENT-RETEST-001` & `SUMI-REL001-REMEDIATION-001` (`HISTORICAL EVIDENCE` — `2026-10-20`)
Recorded in `artifacts/local/session-ops/SUMI-REL001-INDEPENDENT-RETEST-001/` and `artifacts/local/session-ops/SUMI-REL001-REMEDIATION-001/`:
- **Reproduction & Full HSX 2024 Dataset Reconciliation**: Executed `scripts/reproduce_rel001.py` and `scripts/independent_rel001_audit.py` against `docs/CafeF.SolieuGD.Upto05102026/CafeF.HSX.Upto05.10.2026.csv`:
  - Reproduced `HPG` 2024 (`250` source rows -> `228` accepted, `22` rejected prior to fix; `250` accepted, `22` normalized, `0` rejected, `100%` parity with `CafeF.HSX.clean.csv` after fix).
  - Reconciled Core 10 HSX symbols in 2024 (`2,500` source rows -> `2,500` accepted, `202` normalized, `0` rejected, `100%` parity with `CafeF.HSX.clean.csv`).
  - Reconciled all `776` distinct HSX `<Ticker>` symbols (`398` 3-char tickers + `378` CWs; `525` symbols with $\ge 100$ rows) in 2024 (`124,392` source rows, `14,322` raw OHLC violations -> `14,321` (`99.9930%`) safely normalized, `1` (`0.0070%`, `BTP 2024-01-18` `+42.4 VND` / `31.34 bps` corruption) rejected).
- **Real Chromium Browser Verification (`1440×1000`, `6/6` Scenarios Passed in both Remediation & Independent Retest, `0` Console/Page Errors)**:
  - Verified against isolated test DBs (`screenshots/` retained in both session folders).

### 3.1 Checks Executed in Prior Session `SUMI-RELEASE-GATE-001` (`HISTORICAL EVIDENCE` — `2026-10-19`)
The following checks were directly executed in this session (`artifacts/local/session-ops/SUMI-RELEASE-GATE-001/`):
- **Isolated Assessment Database & Process Binding**: Created `test_db/sumi_release_gate_isolated.db` via SQLite `.backup` from `sumi_owner_uat_baseline_snapshot.db`, verified `lsof` binding (`logs/active_db_verification.log`), and confirmed `backend/sumi.db` (`234206fc...`) and `sumi_owner_uat_isolated.db` (`33237401...`) remained 100% untouched (`logs/post_test_db_checksums.log`).
- **5 End-to-End Real Chromium Browser Journeys (`65` Screenshots, `0` Console/Page/500 Errors)**:
  - **Journey 1**: Condition A (`TCB`, `QA1`, duplicate/conflict, malformed, weekend, rollback) + Condition B (`HPG` raw CafeF `SolieuGD` -> identified `REL-001` where `22/250` split-adjusted rows are rejected; `VCB` pre-cleaned CafeF -> `250/250` rows imported cleanly).
  - **Journey 2**: Session `#9` rollback recovery banner; Session `#8` (`FPT`) at `1440×1000`, `1280×800`, and `1024×768` (`<1280px` trade guard); `SMA`, `EMA`, `RSI`, `MACD`, `CCI` lifecycle; `1D -> 1W` switch with `warming (19/20)` badges; drawings (`horizontal`, `text`, `Escape`, `undo`, `redo`, reload persistence); Symbol Switcher to Condition B `VCB` (`Session #10`); `WARM_SYM` warmup replay (`Session #11`).
  - **Journey 3**: Visual Rule Builder (`abc` blocked inline, AST handoff); Single Backtest (`FPT`, `ZERO_PNL_SYM` `G-08`, `FPT, SSI` slices `SQA-014`, Condition B `VCB` & `HPG`); Strategy Battle IS/OOS overlap guard & comparison; Parameter Sweep (`G-12` parity `diff = 0.005 VND < 0.01 VND`); Multi-Phase Batch (`151ms`); 72-Signal Catalog.
  - **Journey 4**: Scanner (`19` signals across `FPT, SSI, VCI, VCB, HPG`), honest unranked disclosure, immutable Scan History snapshot switching (`SQA-015`), `Blind practice` handoff (`Session #34 VCB`), causal `SignalExplanationInspector`, and `TechnicalFlowBBViewer` (`OHLCV_PROXY` badge, `WARM_SYM` `Warming` notice).
  - **Journey 5**: Session `#1` causal order status projection (`SQA-017`) and cross-surface PnL parity (`SQA-018`: `-360,092.063 VND`). Full Condition B `VCB` (`Session #10`) trade & `T+1.5/T+2` settlement lifecycle (`BUY 300` at bar 23 -> fill at `Open[24] = 59,396.7` -> locked at bars 24 & 25 -> released at bar 26 -> `LONG EXIT` modal `DEF-003` -> `SELL 300` fill at `Open[27] = 59,265.0`), independent math audit (`netPnlDiff = 1.47e-9 VND`, `endingCashDiff = 0 VND`), checklist journal, JSON/CSV exports, `/analytics` sample-size gates (`G-09`), `1D/1W` causality (`0` future leaks), and rapid 9-route stability sweep.
- **Final Technical Regression Gate (`VERIFIED CURRENT STATE`)**:
  - **Full Backend `pytest`** (`PYTHONPATH=backend ./.venv/bin/pytest backend tests -q`): `951 passed, 1 skipped in 23.05s` (`logs/pytest_backend_full.log`).
  - **Golden & Verification `pytest`**: `129 passed in 5.43s` (`logs/pytest_golden_cases.log`).
  - **Frontend `Vitest`**: `50 test files passed, 338 tests passed in 10.76s` (`logs/vitest_frontend.log`).
  - **Frontend TypeScript & Build**: `tsc --noEmit` (`0` errors) and `npm run build` (`✓ built in 293ms`, `logs/build_frontend.log`).
  - **Git & DB Safety**: `git diff --check` (`0` issues), `backend/sumi.db` (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`), `3 files changed, 56 insertions(+), 9 deletions(-)` on protected governance files.

### 3.2 Prior Session `CTX-HARDEN-002` & `CTX-BOOTSTRAP-001` Checks (`HISTORICAL EVIDENCE` — `2026-10-19`)
Recorded in [`artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md) and [`artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md):
- Creation and hardening of the 6 session-context files, path existence checks, `/tmp`-default elimination check, and `OWNER-UAT-PREP-001` dry-run walkthrough.

### 3.3 Reported Code Baseline Evidence from Commit `d2d048a` (`HISTORICAL EVIDENCE` — Not Re-Executed in `OWNER-UAT-PREP-001`)
Recorded in [`FINAL_ACCEPTANCE_GATE_REPORT.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_ACCEPTANCE_GATE_REPORT.md) and [`FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md):
- **Full Backend `pytest`**: `950 passed, 1 skipped in 38.87s` (`logs/pytest_full.log`).
- **Financial Golden Cases (`G-01..G-12`)**: `44 passed in 1.12s` (`logs/pytest_golden_cases.log`).
- **Full Frontend `Vitest`**: `50 test files passed, 338 tests passed` (`logs/vitest_full.log`).
- **Frontend TypeScript & Build**: `npx tsc --noEmit` (`0` errors) and `npm run build` (`built in 372ms`, `0` errors).
- **Independent Real-Browser Acceptance Gate**: `19 / 19 INDEPENDENT_REPRODUCED_PASS`, `0` console errors, `0` uncaught page errors across `1440×1000`, `1280×800`, and `1024×768`.

### 3.4 Reported Evidence from Earlier Commits (`HISTORICAL EVIDENCE`)
- **Commit `925a095` (`2026-10-17`)**: `8/8` `DEF-001..006, 008, 012` resolved, `27/27` financial/API checks, `40/40` Chromium checks, `854` BE / `272` FE tests (`artifacts/local/independent-qa-final-retest/`).
- **Commit `bf0bbc6` (`2026-10-17`)**: `101/101` Playwright UAT checks across Journeys 1–5 (`artifacts/local/development-train-05/`).

---

## 4. Production Database & Workspace-Local Test DB Standard (`VERIFIED CURRENT STATE`)

- **Protected Production Database**: `backend/sumi.db`
  - **Byte Size**: `188,416` bytes
  - **SHA-256 Checksum**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`
  - **Rule**: Strictly read-only. Never point state-changing tests, servers, or UAT at `backend/sumi.db`.
- **Pre-Seeded Fixture Database Source (`EXISTS`)**:
  - `artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db` (`1,462,272` bytes, `SHA-256: 33237401c6d20793ca96844a5fc8e13d85c6d504d28c0ad3ea7133aad5641294`)
- **Prepared Isolated Owner UAT Database (`EXISTS` & Verified)**:
  - Active Writable UAT DB: `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db` (`1,462,272` bytes, `SHA-256: 33237401c6d20793ca96844a5fc8e13d85c6d504d28c0ad3ea7133aad5641294`)
  - Clean Reset Snapshot: `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_baseline_snapshot.db`
  - **Important**: Do **not** use `/tmp` as the default UAT or test database location.

---

## 5. Reconciled Document Authority & Known Discrepancies (`VERIFIED CURRENT STATE`)

1. **Three Protected Governance Files Lag Behind `HEAD` (`864e751`) by Two Commits**:
   - `docs/plans/product-completion/DECISION_LOG.md`, `EXECUTION_STATE.md`, and `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md` contain uncommitted modifications from `2026-10-17` (commit `925a095`, `854` BE / `272` FE tests).
   - Commits `d2d048a` (`2026-10-19`, `950` BE / `338` FE tests, `SQA-001..005, 009..018` resolved) and `864e751` (`2026-10-20`, `961` BE / `339` FE tests, `REL-001` CafeF raw data import correctness resolved) occurred after those edits, and all subsequent sessions were prohibited from touching those 3 dirty files.
   - **Reconciliation Rule**: Treat live Git `HEAD` (`864e751`), this `PROJECT_SNAPSHOT.md`, `FINAL_BASELINE_HANDOFF.md`, and `artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/REL001_BASELINE_CLOSURE_REPORT.md` as authoritative for post-`925a095` state, while preserving the 3 dirty governance files untouched until the Product Owner explicitly authorizes updating and committing them.
2. **Deferred Non-Blocking Backlog Items (`DEFERRED` to Post-V3)**:
   - **From `d2d048a` (`FINAL_BASELINE_HANDOFF.md` §5)**: `SQA-006` (`LOW` PracticeRail 4 tabs + pinned SignalInspector), `SQA-007` (`LOW` SettingsModal static badges), `SQA-008` (`LOW` 3s toast stack), `SQA-019` (`LOW` JournalPage raw JSON for `practice_checklist`).
   - **From `925a095` (`EXECUTION_STATE.md` L209–213)**: `DEF-007` (`P2` minor English labels), `DEF-009` (`P2` visual AST builder default), `DEF-010` (`P2` step buttons vs Jump-to-Date input), `DEF-011` (`P2` 7 drawing tools without separate callout icon).
3. **Legacy Windows Links in `docs/INDEX.md`**:
   - `docs/INDEX.md` contains `file:///e:/Workspace/sumi/docs/...` URLs from an earlier Windows checkout; always use [`CONTEXT_INDEX.md`](CONTEXT_INDEX.md).

---

## 6. Pending Owner Decisions (`PENDING OWNER DECISION`)

1. **Execute Manual Product Owner UAT (`OWNER-UAT-SIGNOFF-001`)**: Follow [`artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_RUNBOOK.md`](../../../artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_RUNBOOK.md) to start the backend and frontend against `sumi_owner_uat_isolated.db`, and walk through Journeys 1–5 in [`artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_SCORECARD.md`](../../../artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_SCORECARD.md).
2. **Final V3 Acceptance & Governance Closure Decision**: After completing manual UAT, decide whether to grant `OWNER_ACCEPTED: true` (`ACCEPTED_V3_COMPLETE: true`) and authorize reconciling/committing the 3 lagging governance files (`DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`) and `docs/operations/agent-sessions/*.md`.
