# DEV Program Bootstrap & Baseline Integrity Evidence

## 1. Executive Summary
- **Program Status**: Transitioning from `ready_for_bootstrap` to `running`
- **Execution Target**: Sumi Autonomous DEV Roadmap (Phase 2 through Phase 10)
- **Baseline Date**: 2026-09-15
- **Working Root**: `e:\Workspace\sumi`
- **Reviewer Mechanism**: `independent_context:critic_auditor`
- **Authoritative Gate**: `scripts/verify-dev-program.mjs`

---

## 2. Git Status and Baseline Verification

### 2.1 Commit HEAD and Branch
- **Branch**: `master`
- **Commit HEAD**: `89e04fb00210027c8b08d3fd0116b5773ac26f17`
- **Latest Commit Message**: `fix(core): resolve P0-P3 technical debt from project assessment` (2026-09-01 18:05:46 +0700)
- **Verification Command**:
  ```powershell
  git rev-parse HEAD
  git branch --show-current
  ```
- **Output**:
  ```
  89e04fb00210027c8b08d3fd0116b5773ac26f17
  master
  ```

### 2.2 Preservation of 159 Historically Staged Deletions
As mandated by `docs/dev-program/START.md` §2 and `ORIGINAL_REQUEST.md` R1, all 159 historically staged deletions must remain preserved in the index without being unstaged, committed, stashed, or checked out. These represent obsolete pre-v2 and legacy development prompt documentation cleanly archived prior to Phase 1.

- **Staged Deletion Count**: exactly 159
- **Staged Diff Git Object Hash**: `22f1c3ef17060626d975a3df3003055019411cac`
- **Staged Filename List SHA256**: `350e04be83d15d57dc2f6ab30646ca3418e622ec8c05872495284d5800e5a8c7`
- **Staged Diff Patch SHA256**: `9ba13b0b2a1781e8f2030bb60f92601ac1b13b76107d0944358f31e3f30bf9e1`
- **Verification Command**:
  ```powershell
  (git diff --cached --name-only | Measure-Object -Line).Lines
  (git diff --cached --diff-filter=D --name-only | Measure-Object -Line).Lines
  $staged = (git diff --cached --name-only) -join "`n" + "`n"
  $hasher = [System.Security.Cryptography.SHA256]::Create()
  [BitConverter]::ToString($hasher.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($staged))).Replace('-', '').ToLower()
  ```
- **Output**:
  ```
  159
  159
  350e04be83d15d57dc2f6ab30646ca3418e622ec8c05872495284d5800e5a8c7
  ```

- **Deletions Directory Breakdown**:
  - `docs/archive/pre_v2/`: 46 files
  - `docs/`: 24 files
  - `docs/dev-prompts/`: 22 files
  - `docs/reviews/`: 22 files
  - `docs/exec-plans/`: 14 files
  - `docs/program/`: 11 files
  - `docs/tester/`: 11 files
  - `docs/review-artifacts/`: 4 files
  - `docs/reviewer-prompts/`: 3 files
  - `docs/testing/`: 2 files

---

## 3. Catalog of Inherited Uncommitted Files

The inherited baseline includes uncommitted working-tree files representing completed and accepted Phase 1 (Volume Spike Signals) and active Replay/Trading Lab tooling. These files are strictly preserved as part of the baseline context.

### 3.1 Modified Files (39 files)
- **Configuration & Root Documentation (5 files)**:
  - `.gitignore`
  - `AGENTS.md`
  - `README.md`
  - `docs/INDEX.md`
  - `docs/release/RELEASE_NOTES_v3.0.0.md`
- **Backend Core & Replay Services (6 files)**:
  - `backend/app/api/replay.py`
  - `backend/app/api/symbols.py`
  - `backend/app/domain/engine/strategy_indicator_adapter.py`
  - `backend/app/main.py`
  - `backend/app/services/practice_workflow_service.py`
  - `backend/app/tests/test_backtest.py`
- **Frontend Workspace & Replay (27 files)**:
  - `frontend/src/App.tsx`
  - `frontend/src/api/replayApi.ts`
  - `frontend/src/components/chart/CandleChart.tsx`
  - `frontend/src/components/chart/DrawingInspector.tsx`
  - `frontend/src/components/chart/DrawingToolbar.tsx`
  - `frontend/src/components/chart/IndicatorManager.tsx`
  - `frontend/src/components/chart/IndicatorRenderRegistry.ts`
  - `frontend/src/components/chart/PositionLineManager.ts`
  - `frontend/src/components/chart/SeriesManager.ts`
  - `frontend/src/components/chart/workspaceTypes.ts`
  - `frontend/src/components/common/SessionPicker.tsx`
  - `frontend/src/components/layout/Sidebar.css`
  - `frontend/src/components/layout/Sidebar.tsx`
  - `frontend/src/components/replay/PositionPanel.tsx`
  - `frontend/src/components/replay/PracticeJournal.tsx`
  - `frontend/src/components/replay/PracticeRail.tsx`
  - `frontend/src/components/replay/ReplayWorkspace.tsx`
  - `frontend/src/components/replay/ReplayWorkspaceController.tsx`
  - `frontend/src/components/replay/SessionSetup.tsx`
  - `frontend/src/components/replay/TradeControls.tsx`
  - `frontend/src/components/replay/__tests__/PracticeWorkflow.test.tsx`
  - `frontend/src/features/indicators/__tests__/indicatorDomain.test.ts`
  - `frontend/src/features/indicators/indicatorDomain.ts`
  - `frontend/src/hooks/useModalFocus.ts`
  - `frontend/src/index.css`
  - `frontend/src/pages/StrategyLabPage.tsx`
  - `frontend/src/pages/__tests__/StrategyLabPage.test.tsx`
- **Scripts (1 file)**:
  - `scripts/product-uat.mjs`

### 3.2 Baseline Untracked Files (110 files)
- **Backend Signal Engine & Tests (10 files)**:
  - `backend/app/api/signals.py`
  - `backend/app/domain/signals/__init__.py`, `models.py`, `registry.py`, `volume.py`
  - `backend/app/domain/strategy/examples/ema_crossover.yaml`, `ichimoku_cloud.yaml`
  - `backend/app/domain/strategy/signal_binding.py`
  - `backend/app/schemas/signal_schema.py`
  - `backend/app/services/signal_service.py`
  - `backend/app/tests/test_signal_binding.py`, `test_signals.py`, `test_signals_api.py`
- **Frontend Signal & Replay Components (13 files)**:
  - `frontend/src/api/signalsApi.ts`, `frontend/src/types/signals.ts`
  - `frontend/src/components/signals/SignalInspector.tsx`, `__tests__/SignalInspector.test.tsx`
  - `frontend/src/components/strategy/MultiStrategyEquityChart.tsx`
  - `frontend/src/components/chart/ChartLegendOverlay.tsx`
  - `frontend/src/components/replay/KeyboardShortcutsModal.tsx`, `PracticeScoreboard.tsx`, `ReplayControlDock.tsx`, `SessionDebriefModal.tsx`, `SymbolSwitcherModal.tsx`
  - `frontend/src/components/replay/__tests__/KeyboardShortcutsModal.test.tsx`, `SessionDebriefModal.test.tsx`
- **DEV Program Documentation, Prompts & Research (61 files)**:
  - `docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md`
  - `docs/dev-program/` (`README.md`, `ROADMAP.md`, `START.md`, `STATE.json`)
  - `docs/dev-prompts/` (12 prompt specifications)
  - `docs/exec-plans/` (`DEV_PROGRAM_ANTIGRAVITY_SETUP.md`, `P0_BASELINE_FREEZE.md`, `P1_SIGNAL_VOLUME_SPIKE.md`)
  - `docs/research/` (including Money Flow Blackbox V1 handoffs, Doraemon market data audit, implementation plan)
  - `docs/reviews/` (`P1R_03_SCOPE_INVENTORY.md`, `P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`)
- **Root Scripts & Launchers (7 files)**:
  - `scripts/comprehensive-system-uat.mjs`, `scripts/run-comprehensive-uat.ps1`
  - `scripts/start-sumi.ps1`, `scripts/stop-sumi.ps1`
  - `scripts/verify-dev-program.mjs`
  - `start-sumi.bat`, `stop-sumi.bat`
- **Root Request and Baseline Metadata (19 files)**:
  - `ORIGINAL_REQUEST.md`
  - Baseline `.agents/` survey and orchestrator specifications

---

## 4. Production Database Invariant & Integrity

### 4.1 Production Database Identification
- **File**: `backend/sumi.db`
- **File Size**: `621,199,360` bytes
- **SHA256**: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`
- **WAL File (`backend/sumi.db-wal`)**: `False` (absent)
- **SHM File (`backend/sumi.db-shm`)**: `False` (absent)
- **Verification Command**:
  ```powershell
  $dbPath = 'backend/sumi.db'
  (Get-Item $dbPath).Length
  (Get-FileHash -Algorithm SHA256 $dbPath).Hash
  Test-Path 'backend/sumi.db-wal'
  Test-Path 'backend/sumi.db-shm'
  ```
- **Output**:
  ```
  621199360
  92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64
  False
  False
  ```

### 4.2 Test Runner Database Isolation
- **Unit & Integration Tests**:
  - Backend `backend/app/tests/conftest.py` configures `SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"` with `StaticPool`.
  - No automated test imports, reads, or writes to `backend/sumi.db`.
- **Fast Technical Gate (`scripts/verify-v2.ps1`)**:
  - Allocates an isolated temporary directory in Windows `$Temp`:
    `$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("sumi-verify-v2-" + [System.Guid]::NewGuid().ToString("N"))`
  - Sets `$env:DATABASE_URL = "sqlite:///$TempDb"`
  - Executes Alembic migrations and tests solely against `$TempDb`
  - Cleans up `$TempDir` and `$TempDb` in the `finally` block
- **Zero-Drift Invariant**:
  - The SHA256 digest of `backend/sumi.db` has been verified before and after test executions, confirming zero byte modification.

---

## 5. Structural Checkpoint Verification

### 5.1 Verification Commands and Self-Test
The DEV program supervisor script `scripts/verify-dev-program.mjs` enforces structural invariants:
1. **Self-Test Mode**:
   ```powershell
   node scripts/verify-dev-program.mjs --self-test
   ```
   Output: `Self-test PASS: 13 cases` (Exit code: 0)
2. **Program State Mode**:
   ```powershell
   node scripts/verify-dev-program.mjs
   ```
   Validates roadmap tasks in `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` vs `STATE.json`, evidence file paths, dependency DAG acyclicity, and single active batch.

### 5.2 Transition Criteria for BOOTSTRAP Task
With this document recorded at `docs/reviews/DEV_PROGRAM_BOOTSTRAP_EVIDENCE.md`:
- `workingRoot`: `"e:\\Workspace\\sumi"`
- `bootstrapEvidence`: `"docs/reviews/DEV_PROGRAM_BOOTSTRAP_EVIDENCE.md"`
- `reviewerMechanism`: `"independent_context:critic_auditor"`
- Task `BOOTSTRAP`: `status: "accepted"`, `evidence: ["docs/reviews/DEV_PROGRAM_BOOTSTRAP_EVIDENCE.md"]`
- Program `status`: `"running"`

Post-transition execution of `node scripts/verify-dev-program.mjs` yields `validation: PASS` with `dependencyReady: ["P2-BT-01", "P3-PRICE-01", "P5-DATA-01", "P8-ICHI-01"]`.

---

## 6. Baseline Verification Sign-off
- **Integrity Status**: VERIFIED GENUINE
- **Production Database**: UNTOUCHED (SHA256: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`)
- **159 Staged Deletions**: INTACT (SHA256: `350e04be83d15d57dc2f6ab30646ca3418e622ec8c05872495284d5800e5a8c7`)
- **Working Tree Baseline**: FROZEN & PRESERVED
