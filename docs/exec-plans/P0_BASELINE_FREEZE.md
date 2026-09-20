# P0-BASE-01 — Freeze Implementation Baseline

## Outcome

Establish an authoritative, reproducible, and verifiable baseline record of the Sumi repository prior to any Phase 1 implementation work. This document records the exact Git working tree and index state, snapshot artifact locations and SHA-256 hashes, SQLite database invariant, classification of all 252 tracked/untracked changes, Phase 1 scope and overlap boundaries, and explicit guidance for the reviewer prior to authorizing subsequent batches.

---

## Repository Identity & Metadata

- **Repository Root:** `E:/Workspace/sumi`
- **Current Branch:** `master`
- **Full HEAD SHA:** `89e04fb00210027c8b08d3fd0116b5773ac26f17`
- **Capture Timestamp:** `2026-09-12T20:01:32.4287841+07:00`
- **Local Timezone:** `SE Asia Standard Time` (UTC+07:00)
- **Git Version:** `git version 2.46.2.windows.1`
- **Submodule Status:** None (no git submodules configured)
- **Operating Environment:** Windows (PowerShell shell environment)

---

## Applicable Instructions & Hierarchy

1. **Root Rule Authority:** [AGENTS.md](file:///e:/Workspace/sumi/AGENTS.md) — Sumi repository operating rules, invariants (never leak future candles, backend indicator authority, database immutability during test/UAT, no chart dependency additions without spike, local-first behavior), and verification protocols.
2. **ExecPlan Standard:** [PLANS.md](file:///e:/Workspace/sumi/PLANS.md) — Standard structure, milestones, invariants, and planning rules.
3. **Canonical Implementation Roadmap:** [SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md](file:///e:/Workspace/sumi/docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md) — Authoritative program roadmap, specifically:
   - **Section P.1:** Review verdict, bugs and evidence, locked boundaries.
   - **Section P.2:** Baseline freeze packet mandate.
   - **Section P.3:** Phase 1 bounded vertical scope (`P1-SIG-01 + P1-SIG-02`).
   - **Section P.4:** First-slice contracts, expected candidate files, and protected surfaces.
   - **Section P.7:** Bounded delegation protocol for DEV and reviewer.
4. **Previous Handoff Documentation:** [V3_FINAL_HANDOFF_REPORT_2026-09-12.md](file:///e:/Workspace/sumi/docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md) — Context for V3 Trading Lab and Strategy Tester pre-existing features.
5. **Nested AGENTS.md Search:** A full repository search confirmed that no nested `AGENTS.md` files exist in any subdirectory. Only the root `AGENTS.md` applies.

---

## Initial Git State Summary

At session start, the repository exhibits a significant volume of pre-existing, uncommitted changes stemming from preceding development iterations:

| Metric | Value | Detail |
| :--- | :--- | :--- |
| **Total Porcelain Entries** | 218 entries | Output of `git status --porcelain=v1` |
| **Staged Changes** | 159 files | 100% staged deletions (`D`) in index |
| **Unstaged Tracked Changes** | 40 files | 100% tracked modifications (`M`) in working tree |
| **Untracked Files** | 53 files | Listed via `git ls-files --others --exclude-standard` (19 entries in collapsed porcelain v1) |
| **Untracked Directories** | 2 | `docs/research/` (36 files), `frontend/src/components/strategy/` (1 file) |
| **Renames / Moves** | 0 | None detected |
| **Submodules** | 0 | None present |

> [!IMPORTANT]
> All pre-existing staged and unstaged modifications are treated as user-owned or pre-existing until reviewed. Under no circumstances may an implementation task silently commit, wipe, or overwrite these changes without explicit reviewer authorization.

---

## Detailed Git Inventory & Classification

All repository paths presenting changes or untracked status have been classified into one of four mandatory categories:
- `INTENTIONAL_PROJECT_CHANGE`: Evidenced in documentation, release reports, or authoritative plans.
- `RESEARCH_OR_HANDOFF_ARTIFACT`: Quantitative research, audit reports, or handoff specifications.
- `UNRELATED_OR_PREEXISTING`: Pre-existing code/test/UI modifications from prior iterations.
- `INTENT_UNCONFIRMED`: Actions (such as staged deletions) lacking written authorization or approval evidence.

### 1. Unstaged Tracked Modifications (40 Files)

These 40 files contain active modifications in the working tree relative to the index/HEAD:

| Path | Status | Diff Lines (+/-) | Classification | Context / Rationale |
| :--- | :---: | :---: | :--- | :--- |
| `.gitignore` | ` M` | +1 / -0 | `INTENTIONAL_PROJECT_CHANGE` | Adds `scratch/` to gitignore |
| `AGENTS.md` | ` M` | +46 / -0 | `INTENTIONAL_PROJECT_CHANGE` | Sumi repository operating rules & verification standards |
| `README.md` | ` M` | +163 / -48 | `INTENTIONAL_PROJECT_CHANGE` | V3 documentation, 1-click Windows starter instructions, UAT gate |
| `backend/app/api/replay.py` | ` M` | +7 / -0 | `UNRELATED_OR_PREEXISTING` | Clean controller delegation for replay routes |
| `backend/app/api/symbols.py` | ` M` | +20 / -15 | `UNRELATED_OR_PREEXISTING` | PEP 8 top-level import reorganization |
| `backend/app/domain/engine/strategy_indicator_adapter.py` | ` M` | +28 / -0 | `INTENTIONAL_PROJECT_CHANGE` | Explicitly cited in Section D: raw OHLCV arrays & Ichimoku mapping |
| `backend/app/services/practice_workflow_service.py` | ` M` | +43 / -12 | `UNRELATED_OR_PREEXISTING` | Practice journal, scoreboard, reset workflows |
| `backend/app/services/replay_service.py` | ` M` | +0 / -14 | `UNRELATED_OR_PREEXISTING` | Replay candle navigation cleanup |
| `backend/app/services/trade_lifecycle_service.py` | ` M` | +31 / -100 | `UNRELATED_OR_PREEXISTING` | Practice trade management adjustments (PROTECTED) |
| `backend/app/tests/test_backtest.py` | ` M` | +43 / -2 | `UNRELATED_OR_PREEXISTING` | Backtest test coverage for current strategy engine |
| `backend/app/tests/test_practice_workflow.py` | ` M` | +29 / -2 | `UNRELATED_OR_PREEXISTING` | Practice workflow unit tests |
| `backend/app/tests/test_trade_lifecycle.py` | ` M` | +43 / -43 | `UNRELATED_OR_PREEXISTING` | Trade lifecycle unit tests |
| `docs/INDEX.md` | ` M` | +69 / -15 | `UNRELATED_OR_PREEXISTING` | Documentation index updates |
| `docs/release/RELEASE_NOTES_v3.0.0.md` | ` M` | +37 / -0 | `UNRELATED_OR_PREEXISTING` | Release notes for version 3.0.0 |
| `frontend/src/App.tsx` | ` M` | +31 / -1 | `UNRELATED_OR_PREEXISTING` | Router layout and navigation additions |
| `frontend/src/api/replayApi.ts` | ` M` | +5 / -0 | `UNRELATED_OR_PREEXISTING` | Client API endpoint declarations |
| `frontend/src/components/chart/CandleChart.tsx` | ` M` | +24 / -10 | `UNRELATED_OR_PREEXISTING` | Chart layout, legend overlay integration |
| `frontend/src/components/chart/DrawingInspector.tsx` | ` M` | +26 / -5 | `UNRELATED_OR_PREEXISTING` | Drawing properties inspection |
| `frontend/src/components/chart/DrawingToolbar.tsx` | ` M` | +407 / -25 | `UNRELATED_OR_PREEXISTING` | Professional drawing tools toolbar expansion |
| `frontend/src/components/chart/IndicatorManager.tsx` | ` M` | +107 / -26 | `UNRELATED_OR_PREEXISTING` | Indicator selection modal & param editing |
| `frontend/src/components/chart/IndicatorRenderRegistry.ts`| ` M` | +21 / -5 | `UNRELATED_OR_PREEXISTING` | Multi-pane indicator registry |
| `frontend/src/components/chart/PositionLineManager.ts` | ` M` | +21 / -2 | `UNRELATED_OR_PREEXISTING` | Visual SL/TP lines & percentage formatting fix |
| `frontend/src/components/chart/SeriesManager.ts` | ` M` | +69 / -32 | `UNRELATED_OR_PREEXISTING` | Series management & pane coordination |
| `frontend/src/components/chart/workspaceTypes.ts` | ` M` | +3 / -0 | `UNRELATED_OR_PREEXISTING` | Type definitions for chart workspace |
| `frontend/src/components/layout/Sidebar.css` | ` M` | +92 / -0 | `UNRELATED_OR_PREEXISTING` | Sidebar styling |
| `frontend/src/components/layout/Sidebar.tsx` | ` M` | +62 / -5 | `UNRELATED_OR_PREEXISTING` | Navigation links and active indicators |
| `frontend/src/components/replay/PositionPanel.tsx` | ` M` | +118 / -25 | `UNRELATED_OR_PREEXISTING` | Position panel UI and actions |
| `frontend/src/components/replay/PracticeJournal.tsx` | ` M` | +56 / -8 | `UNRELATED_OR_PREEXISTING` | Practice trade journal component |
| `frontend/src/components/replay/PracticeRail.tsx` | ` M` | +101 / -12 | `UNRELATED_OR_PREEXISTING` | Practice sidebar rail |
| `frontend/src/components/replay/ReplayWorkspace.tsx` | ` M` | +504 / -268| `UNRELATED_OR_PREEXISTING` | Replay layout composition (OVERLAPS PHASE 1) |
| `frontend/src/components/replay/ReplayWorkspaceController.tsx`| ` M`| +141 / -35 | `UNRELATED_OR_PREEXISTING` | Replay navigation & session controller (OVERLAPS PHASE 1)|
| `frontend/src/components/replay/SessionSetup.tsx` | ` M` | +33 / -8 | `UNRELATED_OR_PREEXISTING` | Session setup modal and configuration |
| `frontend/src/components/replay/TradeControls.tsx` | ` M` | +353 / -202| `UNRELATED_OR_PREEXISTING` | Practice trade controls and order entry |
| `frontend/src/components/replay/__tests__/PracticeWorkflow.test.tsx`| ` M`| +28 / -5 | `UNRELATED_OR_PREEXISTING` | Component tests for practice workflow |
| `frontend/src/features/indicators/__tests__/indicatorDomain.test.ts`| ` M`| +2 / -1 | `UNRELATED_OR_PREEXISTING` | Indicator domain unit tests |
| `frontend/src/features/indicators/indicatorDomain.ts` | ` M` | +2 / -0 | `UNRELATED_OR_PREEXISTING` | Indicator parameter bounds and definitions |
| `frontend/src/hooks/useModalFocus.ts` | ` M` | +2 / -1 | `UNRELATED_OR_PREEXISTING` | Accessibility focus hook |
| `frontend/src/index.css` | ` M` | +5 / -0 | `UNRELATED_OR_PREEXISTING` | Global typography and design adjustments |
| `frontend/src/pages/StrategyLabPage.tsx` | ` M` | +223 / -2 | `INTENTIONAL_PROJECT_CHANGE` | Explicitly cited in Section D: presets, equity chart, rating |
| `frontend/src/pages/__tests__/StrategyLabPage.test.tsx` | ` M` | +57 / -0 | `INTENTIONAL_PROJECT_CHANGE` | Tests for StrategyLabPage features |

---

### 2. Staged Deletions (159 Files)

All 159 staged files are marked as deletions (`D`) in the Git index. They encompass historical documentation, past sprint records, development prompts, and legacy execution plans:

| Directory | Count | File Subsets | Classification |
| :--- | :---: | :--- | :--- |
| `docs/` | 32 | `ACCEPTANCE_CRITERIA_V2.md`, `AGENTS.md` (old), `ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`, `AUTONOMOUS_EXECUTION_STATE.md`, `BACKTEST_ENGINE_SPEC.md`, `DECISIONS.md`, `MANUAL_REPLAY_SPEC.md`, `PROGRESS_V2.md`, `SPEC_V2.md`, `V3_ACCEPTANCE_MATRIX.md`, etc. | `INTENT_UNCONFIRMED` |
| `docs/BacktestSample/` | 1 | `Sample.md` | `INTENT_UNCONFIRMED` |
| `docs/archive/pre_v2/` | 7 | `FUTURE_ROADMAP.md`, `IMPLEMENTATION_PLAYBOOK.md`, `PRODUCT_SPEC.md`, `PROJECT_AUDIT_2026-06-26.md`, `QA_ACCEPTANCE_REVIEW_2026-06-28.md`, `README.md`, `SPEC.md` | `INTENT_UNCONFIRMED` |
| `docs/archive/pre_v2/sprints/` | 10 | `sprint_0_environment.md` through `sprint_8_backtest_engine.md`, `sprint_plan_overview.md` | `INTENT_UNCONFIRMED` |
| `docs/dev-prompts/` | 33 | `ANTIGRAVITY_DEV_SESSION_INIT_PROMPT.md`, `BATCH_1`..`5` prompts, `PRO_00`..`12` prompts | `INTENT_UNCONFIRMED` |
| `docs/exec-plans/` | 21 | `BATCH_0`..`5` execution plans, `PRO_00`..`12` execution plans | `INTENT_UNCONFIRMED` |
| `docs/program/` | 11 | `PRO_03`..`12` program plans, `README.md` | `INTENT_UNCONFIRMED` |
| `docs/release/` | 1 | `ACCEPTANCE_MATRIX_AND_LIMITATIONS.md` | `INTENT_UNCONFIRMED` |
| `docs/review-artifacts/2026-07-15/` | 4 | `after-reload.png`, `drawings.png`, `indicators-active.png`, `uat-results.json` | `INTENT_UNCONFIRMED` |
| `docs/reviewer-prompts/` | 3 | `ANTIGRAVITY_REVIEW_SESSION_INIT_PROMPT.md`, `PRO_02_REWORK_01.md`, `REVIEWER_ORCHESTRATOR_HANDOFF_BATCH_3_2026-07-18.md` | `INTENT_UNCONFIRMED` |
| `docs/reviews/` | 24 | `BATCH_1`..`5` reviews, `PRO_02`..`12` reviews across various rounds | `INTENT_UNCONFIRMED` |
| `docs/tester/` | 10 | `01`..`05` reviews, `MANUAL_UAT_TEST_PLAN_AND_PROMPT.md`, `OTHER_FEATURES_REVIEW_AND_GAP_ANALYSIS.md`, `README.md`, `REAL_TRADER_PRODUCT_REVIEW_AND_GAP_ANALYSIS.md`, `SUMI_PROJECT_RESEARCH_AND_REVIEW_PLAN.md` | `INTENT_UNCONFIRMED` |
| `docs/testing/` | 2 | `TRADING_LAB_BUG_AND_UX_REPORT.md`, `TRADING_LAB_TEST_CASES_SPEC.md` | `INTENT_UNCONFIRMED` |

> [!WARNING]
> Staged deletions are classified as `INTENT_UNCONFIRMED` because no explicit decision document or commit message records reviewer acceptance to permanently remove these historical files. They must remain preserved in the index/diff until the reviewer explicitly decides whether to commit or unstage them.

---

### 3. Untracked Files (53 Files)

The 53 untracked files are detailed below:

| Directory / File | Count | Classification | Context & Evidence |
| :--- | :---: | :--- | :--- |
| `backend/app/domain/strategy/examples/ema_crossover.yaml` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Documented in Section D of Plan: new strategy example for EMA crossover |
| `backend/app/domain/strategy/examples/ichimoku_cloud.yaml` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Documented in Section D of Plan: new strategy example for Ichimoku |
| `docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Authoritative handoff report for Sumi V3 |
| `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | The master program plan governing all future DEV batches |
| `docs/research/SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Authoritative system functional and technical specification |
| `docs/research/SUMI_IMPLEMENTATION_AUDIT_REPORT.md` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Detailed architectural audit report |
| `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Market data provider audit |
| `docs/research/data_capability_matrix.csv` | 1 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Market data capability matrix |
| `docs/research/SUMI_Money_Flow_Blackbox_V1_DataReady_Handoff_v3/` | 16 | `RESEARCH_OR_HANDOFF_ARTIFACT` | V3 DataReady handoff suite (`00` through `12`, `MANIFEST`, `MASTER_SPEC`, `README`) |
| `docs/research/SUMI_Money_Flow_Blackbox_V1_Handoff/` | 14 | `RESEARCH_OR_HANDOFF_ARTIFACT` | Earlier Money Flow handoff suite (`01` through `12`, `MASTER_SPEC`, `README`) |
| `frontend/src/components/chart/ChartLegendOverlay.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Chart legend overlay component created during V3 handoff |
| `frontend/src/components/replay/KeyboardShortcutsModal.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Replay shortcuts modal documented in V3 handoff & README |
| `frontend/src/components/replay/PracticeScoreboard.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Real-time practice scoreboard documented in V3 handoff & README |
| `frontend/src/components/replay/ReplayControlDock.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Replay bottom control dock documented in V3 handoff & README |
| `frontend/src/components/replay/SessionDebriefModal.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Session debrief modal documented in V3 handoff & README |
| `frontend/src/components/replay/SymbolSwitcherModal.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Quick symbol switcher modal documented in V3 handoff & README |
| `frontend/src/components/replay/__tests__/KeyboardShortcutsModal.test.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Tests for keyboard shortcuts modal |
| `frontend/src/components/replay/__tests__/SessionDebriefModal.test.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Tests for session debrief modal |
| `frontend/src/components/strategy/MultiStrategyEquityChart.tsx` | 1 | `INTENTIONAL_PROJECT_CHANGE` | SVG multi-strategy equity curve component cited in Section D |
| `scripts/comprehensive-system-uat.mjs` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Full Playwright E2E UAT suite (31 scenarios) cited in AGENTS.md & README |
| `scripts/run-comprehensive-uat.ps1` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Windows wrapper for Playwright UAT cited in AGENTS.md & README |
| `scripts/start-sumi.ps1` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Windows service starter script |
| `scripts/stop-sumi.ps1` | 1 | `INTENTIONAL_PROJECT_CHANGE` | Windows service stopper script |
| `start-sumi.bat` | 1 | `INTENTIONAL_PROJECT_CHANGE` | 1-click Windows starter batch script |
| `stop-sumi.bat` | 1 | `INTENTIONAL_PROJECT_CHANGE` | 1-click Windows stopper batch script |

---

## Snapshot Artifacts & Hashes

To guarantee reproducible comparison and recovery, complete snapshots of the index, diffs, untracked files, metadata, and database state were captured into a dedicated Git-ignored directory.

- **Snapshot Directory:** `scratch/baseline-freeze-20260912-200500/`
- **Absolute Path:** `E:\Workspace\sumi\scratch\baseline-freeze-20260912-200500`
- **Git Ignore Verification:** Confirmed ignored via `git check-ignore scratch/baseline-freeze-20260912-200500/` (matches `.gitignore` line 29: `scratch/`).

| Snapshot Artifact | Size (Bytes) | SHA-256 Checksum | Purpose / Contents |
| :--- | :---: | :--- | :--- |
| `porcelain-status.txt` | 10,919 | `74A5355CAFF718845865E7EF662B5C293E80C3648DFAD2A880C73D42B289E460` | Full `git status --porcelain=v1` output |
| `porcelain-v2-status.txt` | 35,768 | `7EEF60D446C3441F968DECF271CB9871481B977263794ADF426461BF3A947EA6` | Full `git status --porcelain=v2 --branch --untracked-files=all` |
| `staged-diff.patch` | 2,160,736 | `F49D4494A3F62A44412DF2CF8B321BFE849B90EAEB0138D9AC08CB266B9596B4` | Full binary diff of index (`git diff --cached --binary`) |
| `unstaged-diff.patch` | 236,909 | `85C2F4C3F128D84F0869A947296B43B1BE4CD5B459EEE62862A67C91FED38D69` | Full binary diff of working tree (`git diff --binary`) |
| `untracked-files.txt` | 3,624 | `9721BD5504261B531BB761D71AA54C422F4FFE3AF9AEBD9EC8D192644F8C0466` | List of all 53 untracked files from `git ls-files --others` |
| `sha256-manifest.txt` | 13,410 | `00B7D3B28147FF8ED19A21F1183D23BDB81A1F6492670D192281A7453E588B06` | SHA-256 hashes of all 40 modified files and 53 untracked files |
| `metadata.json` | 436 | `A08D14058B1FBAE8DA9BEB470DE9BCC19E915CFFD0793B1815163588639BB1C9` | HEAD SHA, branch, Git version, capture timestamp, timezone |
| `database-hash.json` | 1,220 | `6020C1225274CDE58848B67A8157EB0B48584DD34D04E67ACF1F21C38C36E883` | Filesystem metadata and SHA-256 for `sumi.db`, WAL, and SHM |

---

## Database Invariant & Integrity Record

In accordance with Step D, filesystem metadata and cryptographic checksums were obtained in a strict read-only manner:
- No Sumi server or Python application processes were imported or executed.
- No SQLite database connection was established.
- No checkpoint, migration, or seeding was triggered.

```json
{
  "database": {
    "path": "backend/sumi.db",
    "exists": true,
    "size_bytes": 621199360,
    "last_write_time_utc": "2026-09-05T13:43:46.0000000Z",
    "sha256": "92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64"
  },
  "wal": {
    "path": "backend/sumi.db-wal",
    "exists": false
  },
  "shm": {
    "path": "backend/sumi.db-shm",
    "exists": false
  }
}
```

> [!NOTE]
> As recorded in the technical spec, the SHA-256 of the main SQLite database file does not represent a consistent snapshot if an active WAL file exists or if an external writer is operating concurrently. Here, both `backend/sumi.db-wal` and `backend/sumi.db-shm` are confirmed non-existent (`exists: false`), and no writer is active. The hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` is thus authoritative for the quiescent state.

---

## Phase 1 Scope & Overlap Analysis

Phase 1 (`P1-SIG-01 + P1-SIG-02`) scope is strictly bounded to:
1. Signal domain models, registry, and shared candle-volume feature extraction.
2. Causal, explainable Volume Spike (`volume.relative_volume`, `volume.spike`) end-to-end.
3. Isolated safe Strategy DSL resolution of precomputed signal fixtures.
4. Minimal replay-scoped configuration and explanation UI panel.

### Table 1: Expected New Paths (`EXPECTED_NEW_PATHS`)

All planned candidate files for Phase 1 were verified against the filesystem and Git status:

| Candidate Path | Exists Now? | Git Status | Classification | Overlap with Pre-existing? | Phase 1 Authorization Status |
| :--- | :---: | :---: | :--- | :---: | :--- |
| `backend/app/domain/signals/__init__.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/domain/signals/models.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/domain/signals/registry.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/domain/signals/volume.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/domain/strategy/signal_binding.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/schemas/signal_schema.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/services/signal_service.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/api/signals.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/tests/test_signals.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/tests/test_signal_binding.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `backend/app/tests/test_signals_api.py` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `frontend/src/api/signalsApi.ts` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `frontend/src/types/signals.ts` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `frontend/src/components/signals/SignalInspector.tsx` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |
| `frontend/src/components/signals/__tests__/SignalInspector.test.tsx` | **False** | None | `PLANNED_PHASE_1_MODULE` | **None** | Allowed to create in Phase 1 |

*Finding:* None of the 15 candidate files exist in the repository. There is zero namespace collision or uncommitted drift on these planned paths.

---

### Table 2: Existing Integration Paths (`EXISTING_INTEGRATION_PATHS`)

Phase 1 requires integrating with a small number of existing production surfaces:

| Integration Path | Exists Now? | Git Status | Classification | Overlap with Pre-existing? | Reviewer Resolution Required? |
| :--- | :---: | :---: | :--- | :---: | :--- |
| `backend/app/main.py` | **True** | Clean | `PRODUCTION_ROUTER_ENTRY` | **None** (Clean at HEAD) | **No.** Phase 1 is allowed to add router inclusion once authorized. |
| `frontend/src/components/replay/ReplayWorkspace.tsx` | **True** | ` M` | `UNRELATED_OR_PREEXISTING` | **YES (504 lines modified)** | **YES.** Pre-existing modifications in ReplayWorkspace. Reviewer must approve whether Phase 1 mounts its panel on top of this dirty file or whether baseline should be staged/committed first. |
| `frontend/src/components/replay/ReplayWorkspaceController.tsx` | **True** | ` M` | `UNRELATED_OR_PREEXISTING` | **YES (141 lines modified)** | **No.** Read-only inspection only in Phase 1 (session/index state); Phase 1 must not edit this file. |
| `scripts/run-comprehensive-uat.ps1` | **True** | `??` | `INTENTIONAL_PROJECT_CHANGE` | **YES (Untracked file)** | **YES.** Reviewer must confirm that Phase 1 can execute this script as a verification gate. |
| `scripts/comprehensive-system-uat.mjs` | **True** | `??` | `INTENTIONAL_PROJECT_CHANGE` | **YES (Untracked file)** | **YES.** Reviewer must decide whether Phase 1 adds Volume Spike scenarios here or in a standalone test script. |

---

## Protected Surfaces (Strict Prohibition)

The following components and files are strictly protected from modification during Phase 1:

1. `backend/app/services/backtest_service.py` (Must not connect signals to legacy backtest execution).
2. `backend/app/services/trade_lifecycle_service.py` (Practice trade execution lifecycle is completely separate).
3. `backend/app/domain/strategy/rule_evaluator.py` (Safe AST whitelist must not be edited or widened).
4. `backend/app/models/` (Strategy persistence schemas and database ORM models are protected).
5. `backend/app/domain/providers/` (Market data provider code must not be modified).
6. `backend/alembic/` (Database migrations are prohibited in Phase 1).
7. `frontend/src/components/chart/` (Chart provider adapters are protected).
8. `backend/sumi.db` (Production database is strictly read-only).

---

## Unknown Ownership Requiring Reviewer Attention

The following items have unconfirmed intent and require reviewer adjudication:

1. **159 Staged Deletions:**
   - A large set of documentation, sprint history, dev prompts, and previous review files were staged for deletion prior to this session.
   - *Reviewer Decision Needed:* Determine whether to commit these deletions to finalize repository cleanup, or to unstage (`git reset HEAD`) to preserve historical records before Phase 1 commences.
2. **Pre-existing Working Tree Modifications (40 Files):**
   - Major enhancements to Trading Lab, Drawing Toolbar, and Strategy Tester exist as unstaged modifications.
   - *Reviewer Decision Needed:* Determine whether these modifications should be committed as a dedicated V3 completion commit, stashed, or retained as an active working tree base for Phase 1.
3. **Integration Point Overlap on `ReplayWorkspace.tsx`:**
   - Phase 1 expects to mount `SignalInspector` within the replay page. Since `ReplayWorkspace.tsx` already has 504 modified lines, parallel edits carry high risk of merge conflicts if not carefully isolated.
   - *Recommendation:* Phase 1 should implement `SignalInspector` as a self-contained component and mount it via a minimal, non-destructive import in `ReplayWorkspace.tsx`.

---

## Final Git State & Stability Verification

Following the generation of snapshot artifacts and the creation of this document, a full verification was executed:

1. **Working Tree Drift Check:**
   - No production files were modified by this task.
   - The snapshot directory `scratch/baseline-freeze-20260912-200500/` was created inside `.gitignore` and does not appear in `git status`.
   - The only new file in the repository is:
     `docs/exec-plans/P0_BASELINE_FREEZE.md` (untracked, allowed).
2. **Database Immutability Check:**
   - SHA-256 of `backend/sumi.db` re-verified: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (Identical).
   - WAL and SHM remain non-existent.
3. **Index State:**
   - Staging area was untouched (exactly 159 staged deletions remain).

---

## Stability Verdict

### **`BASELINE_READY_FOR_REVIEW`**

**Verdict Rationale:**
- The repository baseline has been captured with complete determinism and cryptographic integrity.
- Working tree, index, and database hashes are fully preserved without drift during the freeze operation.
- Complete snapshot artifacts and SHA-256 manifests are safely archived in an ignored directory (`scratch/baseline-freeze-20260912-200500/`).
- Phase 1 candidate paths are verified completely clean with no collisions.
- Specific overlap points (`ReplayWorkspace.tsx`, UAT scripts) have been isolated and documented.

> [!CAUTION]
> This verdict certifies that the **baseline is frozen and ready for reviewer inspection**. It does NOT constitute `PHASE_1_AUTHORIZED`. Implementation of Phase 1 must await formal reviewer authorization.

---

## Recommended Reviewer Action

1. Inspect this baseline document and the corresponding snapshot directory `scratch/baseline-freeze-20260912-200500/`.
2. Adjudicate the disposition of the 159 staged deletions and 40 unstaged modifications (e.g. commit baseline as a milestone commit or authorize DEV to build on top of the dirty working tree).
3. Authorize Phase 1 (`P1-SIG-01 + P1-SIG-02`) using the copyable delegation prompt defined in Section P.7 of `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`.

---

## Reviewer Adjudication — 2026-09-12

### Snapshot correction

The initial packet listed and hashed all 53 original untracked files but did not retain their contents. The staged and unstaged Git patches cannot recover untracked files. Reviewer inspection therefore added a path-preserving archive inside the already ignored snapshot directory:

| Artifact | Entries | Unique paths | SHA-256 | Verification |
| :--- | ---: | ---: | :--- | :--- |
| `scratch/baseline-freeze-20260912-200500/untracked-content-path-preserving.zip` | 53 | 53 | `A54CA653D4F7088E1F7908B3863234565A9A612D770B3980E71C231385199F96` | Extracted into a temporary directory; all 53 file hashes matched the original `UNTRACKED` entries in `sha256-manifest.txt` |

This correction created no tracked change other than this reviewer appendix. Temporary verification content was removed after checking. The original source files, staging area, production database and application code were not changed.

### Decisions for Phase 1

1. **Staged deletions:** leave all 159 staged deletions exactly as captured. Do not commit, unstage, restore or include them in Phase 1 review. They are unrelated preserved state and do not block Phase 1.
2. **Other dirty files:** build on the captured current checkout. Do not commit or stash the 40 pre-existing modifications. Phase 1 review uses the baseline artifacts and a path-scoped incremental diff.
3. **Replay UI overlap:** Phase 1 may make a minimal additive edit to `frontend/src/components/replay/ReplayWorkspace.tsx`, whose reviewed pre-Phase-1 SHA-256 is `6B58401E8A3D868106CABA22F53D7DAA8D1905A282BD79C2E3A6E695579B486F`. The only intended changes are the `SignalInspector` import and one mount at the top of the existing `PracticeRail` trade content, after detailed `ScannerSourceContext` and before `practiceData`. Any structural rewrite or formatting churn is prohibited. `ReplayWorkspaceController.tsx` remains read-only.
4. **UAT overlap:** run the captured `scripts/run-comprehensive-uat.ps1` unchanged. Do not edit it or `scripts/comprehensive-system-uat.mjs`. Add focused Phase 1 browser assertions to clean tracked `scripts/product-uat.mjs`, and run them through existing `scripts/run-product-uat.ps1`. Test changes must be identifiable in the incremental diff.
5. **Database:** production hash remains `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`; Phase 1 tests/UAT use temporary databases only.

### Reviewer verdict

`BASELINE_ACCEPTED_FOR_PHASE_1`

This verdict authorizes only the bounded `P1-SIG-01 + P1-SIG-02` batch defined in Sections O and P of the final implementation plan. It does not authorize Phase 2, Git cleanup, committing, migrations, provider changes, backtest integration or P&L evaluation.
