# P1R-03 — Incremental Scope & Drift Inventory

**Date:** 2026-09-15 (Updated at P1R-03D)  
**Status:** COMPLETE (INVENTORY & SCOPE CLOSURE — NO UNRELATED DRIFT CLEANED)  
**Authority:** `docs/dev-prompts/P1R_03A_SCOPE_INVENTORY.md`, `docs/dev-prompts/P1R_03B_UAT_SCOPE_CLOSURE.md`, `docs/dev-prompts/P1R_03C_UAT_EVIDENCE_DETERMINISM.md` & `docs/dev-prompts/P1R_03D_REMOVE_EVIDENCE_SUPPRESSION.md`  
**Reference Baseline:** `scratch/baseline-freeze-20260912-200500/`  
**Git HEAD:** `89e04fb00210027c8b08d3fd0116b5773ac26f17` (branch `master`)  
**Verdict / Action:** Phase 1 bounded scope quarantined; unrelated drift preserved separately; Phase 2 not authorized.

---

## 1. Executive Summary & Five-Snapshot Untracked Progression

This inventory establishes an exact cryptographic and semantic comparison between the Sumi working tree as of checkpoint P1R-02B / P1R-03D and the frozen baseline recorded in `scratch/baseline-freeze-20260912-200500/`.

To eliminate ambiguity regarding documentation timing versus source drift, repository untracked counts are explicitly partitioned across five sequential measurement points:

| Metric | Frozen Baseline (2026-09-12 20:05) | P1R-03A Pre-Report (2026-09-13 22:51) | P1R-03A Post-Report (2026-09-13 22:55) | P1R-03B Start State (2026-09-13 23:14) | P1R-03C Start State (2026-09-14 22:08) | P1R-03D Start State (2026-09-14 23:10) | Delta / State |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| **Git Branch / HEAD** | `master` / `89e04fb00210...` | `master` / `89e04fb00210...` | `master` / `89e04fb00210...` | `master` / `89e04fb00210...` | `master` / `89e04fb00210...` | `master` / `89e04fb00210...` | Bitwise Identical |
| **Staged Entries** | 159 deletions (`D`) | 159 deletions (`D`) | 159 deletions (`D`) | 159 deletions (`D`) | 159 deletions (`D`) | 159 deletions (`D`) | **Exact Match (0 drift)** |
| **Unstaged Tracked Changes** | 40 files | 43 files | 43 files | 43 files | 43 files | 43 files | +3 clean-at-HEAD files |
| **Baseline Manifest Entries** | 93 paths | 93 paths verified | 93 paths verified | 93 paths verified | 93 paths verified | 93 paths verified | **80 Unchanged, 13 Changed, 0 Missing** |
| **Untracked Total (`ls-files`)** | 53 files | 80 files | 81 files | 82 files | 83 files | 84 files | Governed by doc creation |
| **New Untracked Files** | 0 files | 27 files | 28 files | 29 files | 30 files | 31 files | Partitioned below |
| ↳ *Phase 1 Code/Test Files* | 0 files | **15 files** | **15 files** | **15 files** | **15 files** | **15 files** | **Exact 15 candidates (0 extra code files)** |
| ↳ *Reviewer / Governance Docs* | 0 files | **12 files** | **13 files** (+P1R-03 report) | **14 files** (+P1R-03B prompt) | **15 files** (+P1R-03C prompt) | **16 files** (+P1R-03D prompt) | Documentation timing only |
| **Production DB SHA-256** | `92A7F65AB8B7BB...` | `92A7F65AB8B7BB...` | `92A7F65AB8B7BB...` | `92A7F65AB8B7BB...` | `92A7F65AB8B7BB...` | `92A7F65AB8B7BB...` | **Bitwise Identical (0 mutation)** |
| **DB WAL / SHM Status** | Absent (`exists: false`) | Absent (`exists: false`) | Absent (`exists: false`) | Absent (`exists: false`) | Absent (`exists: false`) | Absent (`exists: false`) | **Strict Invariant Maintained** |

---

## 2. Git Working Tree & Index State Verification

### 2.1 Staged Deletions (159 Files)
- The Git index contains exactly **159 staged deletions (`D`)** and **0 additions, modifications, or renames**.
- Comparison with `scratch/baseline-freeze-20260912-200500/staged-diff.patch` and `porcelain-status.txt` confirms:
  - **100% identity** of the staged deletion set.
  - Zero files have been unstaged, committed, or additionally staged.
  - Preserved historical documentation and legacy test files remain cleanly quarantined in the index.

### 2.2 Unstaged Modifications (43 Files)
- Baseline contained 40 unstaged modified files relative to HEAD.
- Current checkout contains 43 unstaged modified files relative to HEAD.
- The 3 new unstaged files were clean at HEAD during baseline freeze:
  1. `backend/app/main.py`: router registration for signals API (+2 / -1).
  2. `frontend/src/components/common/SessionPicker.tsx`: refetch effect and badge formatting (+8 / -2).
  3. `scripts/product-uat.mjs`: Phase 1 Signal Inspector assertions and selector resilience tweaks (+306 / -9).

---

## 3. Baseline Manifest Audit (93 Paths)

All 93 paths recorded in `scratch/baseline-freeze-20260912-200500/sha256-manifest.txt` were cryptographically hashed from the current working tree and compared against baseline expectations:

- **Unchanged (80 Paths):** 29 tracked files + 51 untracked files remain 100% bitwise identical to the baseline freeze.
- **Changed After Freeze (13 Paths):** 11 tracked files + 2 untracked files exhibit modifications.
- **Missing (0 Paths):** Zero baseline files have been deleted or moved.

### 3.1 Detail of 13 Baseline Files Changed Post-Freeze

| Category | Path | Baseline SHA-256 | Current Working Tree SHA-256 | Size (Base -> Cur) |
| :--- | :--- | :---: | :---: | :---: |
| `UNSTAGED_MODIFIED` | `AGENTS.md` | `C38BCCBAA1FD1418F6F41A25DB79446E46B5F609BFE6E8A63DE3F98604603366` | `F4D6389DA9CBBEC673C1937AC30CCC550779C3D18D9209AA771A7E23E121A35B` | 4,014 -> 4,651 |
| `UNSTAGED_MODIFIED` | `backend/app/services/practice_workflow_service.py` | `1FE0C191FA2CBB23A01F694EE426A28788981E651EF4CF6E1A92017FA9F6E64A` | `CE05D90FAFA7E7F0267CF89D904C2E17CE3E83DDC76976D448DB5CEE172F041E` | 15,348 -> 16,800 |
| `UNSTAGED_MODIFIED` | `backend/app/services/replay_service.py` | `B281BE027139DF84F5F4BB5C794B53CE28546BD08C797CAB031CB304C6D444D4` | `D51761B289689F2064819F66A6768C10705C607F6EEA1FC8CDCAE68987D9DCC0` | 12,878 -> 13,181 |
| `UNSTAGED_MODIFIED` | `backend/app/services/trade_lifecycle_service.py` | `7BB74A58CC43C85D98FC59B03396391A29CBDC9AAD125707837B8BB52D95D609` | `B70323B31C60CE83D165B9DA8CEBAB68DB7794541C35A7AE5AE0D74545CCC389` | 23,733 -> 25,809 |
| `UNSTAGED_MODIFIED` | `backend/app/tests/test_practice_workflow.py` | `6B3FE4FFA34A8AC4DD6BC698E91042303D48E7979359C829BBC073F7E6120E18` | `300554F9D2B71B03763C4C940C6BD053E8EEE77CC5112B88CE432737D3D89A23` | 18,532 -> 18,611 |
| `UNSTAGED_MODIFIED` | `backend/app/tests/test_trade_lifecycle.py` | `3EEF0F1BFF5DED4DE4870C1F4622DD49EB7D93C5F09D3340D03C9F84C277C15C` | `EC31B0837BBC3954ABE97BA384B807488A08A221284E080216EE74105F0C947C` | 29,294 -> 26,890 |
| `UNSTAGED_MODIFIED` | `frontend/src/App.tsx` | `D4F2AB3D0026F08558B01EE22D7CBCAF4AD79ABF65B8BC7DF6E768FD5B6EF501` | `464DAF9FAF3DAE87C72D64F57426CFAA52B444243EF970DA8378B698C96FF926` | 3,353 -> 3,447 |
| `UNSTAGED_MODIFIED` | `frontend/src/components/layout/Sidebar.tsx` | `8264B567530C05B86FE248DA8DA793FD07C2C5CB38414FA6666E1420EA68F78F` | `50B5126E0966BD1244F31572AD7B1AC7FE63BC3520EBD840A38543F7FFDC24FC` | 3,007 -> 3,298 |
| `UNSTAGED_MODIFIED` | `frontend/src/components/replay/PracticeRail.tsx` | `09E4DB3B5C805DB9658C9C252AB8F7562C390888C3A2E5E6200CF295A244BCA8` | `130992F301D89F61DE2EE79BF1962A69D7DBB32206C835B7DF1884BB6BBE8973` | 3,808 -> 4,752 |
| `UNSTAGED_MODIFIED` | `frontend/src/components/replay/ReplayWorkspace.tsx` | `6B58401E8A3D868106CABA22F53D7DAA8D1905A282BD79C2E3A6E695579B486F` | `CADE37B371DC4E301C674153ABFE1E1C362107A7AFA774D6C2B7BDFBD1D64094` | 25,177 -> 27,327 |
| `UNSTAGED_MODIFIED` | `frontend/src/components/replay/TradeControls.tsx` | `9405D7226FD3C166187B7F4A6DE80625E296E44361A767C7544FDB765BCD000F` | `6582305E5D6023A74259391EB6261405A3BB7B9D562359FE0697F12A63094A70` | 14,315 -> 19,625 |
| `UNTRACKED` | `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` | `14D6EA4C239EDEE8339CA9E09E057B85E1C4F4E1068868B0541589D86FCF7DE9` | `837C58D7BA1DDDB9301CD828BB8B813877C878455F67036091C36417EAA1F52C` | 26,031 -> 27,244 |
| `UNTRACKED` | `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` | `8624A6F84AD51AAD5F560FFED6097503D82DA58FBDA076573AA7A39AE4B0DD67` | `EDAA62BC4D75FF51F22B71599F3FCC07015980638CBF53BF2D6FB588754A57A0` | 105,118 -> 106,502 |

---

## 4. Untracked Files Inventory & Progression

The 30 new untracked files present at the start of P1R-03C partition cleanly into:
1. **15 Phase 1 Production/Test Modules:** Strictly authorized candidates.
2. **15 Reviewer & Governance Documents:** Operating rules, execution plans, review reports, and prompt contracts.
3. **0 Extra Code Files:** Zero unexpected source, test, or binary files.

### 4.1 Phase 1 Production & Test Files (15 Files)

| Path | Category | Purpose & Bounded Role |
| :--- | :--- | :--- |
| `backend/app/domain/signals/__init__.py` | Domain | Signal package init export |
| `backend/app/domain/signals/models.py` | Domain | `SignalValueType`, `SignalQuality`, `SignalPoint`, `SignalOutputPoint` |
| `backend/app/domain/signals/registry.py` | Domain | `SignalRegistry`, parameter definition, canonical hash |
| `backend/app/domain/signals/volume.py` | Domain | `volume.relative_volume` and `volume.spike` pure calculations |
| `backend/app/domain/strategy/signal_binding.py` | Domain / Strategy | Isolated DSL AST evaluator adapter with exact bar t-1 cross resolution |
| `backend/app/schemas/signal_schema.py` | API Schema | Pydantic v2 schemas: `SignalComputeRequest`, `BatchSignalComputeRequest` |
| `backend/app/services/signal_service.py` | Service | Replay prefix signal computation, session current_index boundary enforcement |
| `backend/app/api/signals.py` | API Controller | `/api/signals/registry` and `/api/signals/replay/{session_id}/calculate` |
| `backend/app/tests/test_signals.py` | Test (Backend) | Core unit tests (RVOL, Spike, zero volume, registry hash, NaN/Inf bounds) |
| `backend/app/tests/test_signal_binding.py` | Test (Backend) | DSL binding tests (AND, OR, Crosses, previous-bar adjacency) |
| `backend/app/tests/test_signals_api.py` | Test (Backend) | Integration tests in temporary DB (replay prefix, rewind cap, error handling) |
| `frontend/src/api/signalsApi.ts` | API Client | Frontend client for registry and calculate endpoints |
| `frontend/src/types/signals.ts` | Frontend Types | TypeScript interfaces matching backend contracts |
| `frontend/src/components/signals/SignalInspector.tsx` | Component | Reactive inspector panel with exact token/generation matching and strict ISO parser |
| `frontend/src/components/signals/__tests__/SignalInspector.test.tsx` | Test (Frontend) | Vitest component tests (ABA protection, strict multiplier, timestamp validation) |

### 4.2 Reviewer & Governance Documents (16 Files at P1R-03D Start)

| Path | Present in P1R-03A Pre? | Present in P1R-03A Post? | Present in P1R-03B Start? | Present in P1R-03C Start? | Present in P1R-03D Start? | Purpose |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| `docs/dev-prompts/README.md` | Yes | Yes | Yes | Yes | Yes | DEV prompt entry point and current authorization state |
| `docs/dev-prompts/P1_01_CORE.md` | Yes | Yes | Yes | Yes | Yes | Initial Phase 1 prompt (M0 + M1) |
| `docs/dev-prompts/P1R_01_BACKEND_CONTRACT_HARDENING.md` | Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-01 (backend null/schema/overflow) |
| `docs/dev-prompts/P1R_01A_SNAPSHOT_INTEGRITY.md` | Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-01A (binding snapshot integrity) |
| `docs/dev-prompts/P1R_01B_NUMERIC_SERIALIZATION_BOUNDARY.md`| Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-01B (serialization boundary) |
| `docs/dev-prompts/P1R_02_UI_RESPONSE_IDENTITY.md` | Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-02 (UI token/stale response) |
| `docs/dev-prompts/P1R_02A_RENDER_IDENTITY.md` | Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-02A (render identity & exact multiplier) |
| `docs/dev-prompts/P1R_02B_TIMESTAMP_VALIDATION.md` | Yes | Yes | Yes | Yes | Yes | Reviewer repair prompt P1R-02B (local strict daily date parser) |
| `docs/dev-prompts/P1R_03A_SCOPE_INVENTORY.md` | Yes | Yes | Yes | Yes | Yes | Scope inventory authorization prompt |
| `docs/dev-prompts/P1R_03B_UAT_SCOPE_CLOSURE.md` | **No** | **No** | **Yes** | Yes | Yes | Authorized UAT & scope closure prompt |
| `docs/dev-prompts/P1R_03C_UAT_EVIDENCE_DETERMINISM.md` | **No** | **No** | **No** | **Yes** | Yes | Authorized UAT evidence determinism prompt |
| `docs/dev-prompts/P1R_03D_REMOVE_EVIDENCE_SUPPRESSION.md` | **No** | **No** | **No** | **No** | **Yes** | Authorized UAT evidence suppression removal prompt |
| `docs/exec-plans/P0_BASELINE_FREEZE.md` | Yes | Yes | Yes | Yes | Yes | Authoritative record of baseline freeze snapshot |
| `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md` | Yes | Yes | Yes | Yes | Yes | ExecPlan for Phase 1 vertical capability |
| `docs/reviews/P1R_03_SCOPE_INVENTORY.md` | **No** | **Yes** | **Yes** | Yes | Yes | This scope inventory document |
| `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md` | Yes | Yes | Yes | Yes | Yes | Master independent reviewer verdict with all repair addenda |

---

## 5. Comprehensive Hunk-by-Hunk Audit & Classification

Each of the 16 files modified post-freeze was compared against its reconstructed baseline state:
- Tracked files were reconstructed by applying the exact CP437-encoded `unstaged-diff.patch` to Git HEAD.
- Untracked files were extracted from `untracked-content-path-preserving.zip`.
- Clean-at-HEAD files were extracted from `git show HEAD:<path>`.

### 5.1 Classification Categories
- **`AUTHORIZED_P1_INTEGRATION`**: Directly required and authorized for Phase 1 Volume Spike integration.
- **`REVIEWER_GOVERNANCE`**: Process, operating rules, or audit guidance added by the reviewer. Must be kept separate from the Phase 1 product patch.
- **`OUT_OF_SCOPE_DRIFT`**: Pre-existing or concurrent modifications belonging to Trading Lab, Drawing Toolbar, or Navigation. Preserved separately; not accepted as P1; not physically cleaned.

### 5.2 File-by-File Classification & Recommendation Table

| File | Hunks | Added / Deleted | Classification | Reviewer Recommendation | Summary of Changes |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `backend/app/main.py` | 1 | +2 / -1 | `AUTHORIZED_P1_INTEGRATION` | **`KEEP_AS_P1`** | Includes `signals.router` at `/api/signals`. |
| `frontend/src/components/replay/ReplayWorkspace.tsx` (Hunks 1, 2, 11) | 3 | +11 / -0 | `AUTHORIZED_P1_INTEGRATION` | **`KEEP_AS_P1`** | Imports and mounts `<SignalInspector>` with `currentTimestamp` inside `PracticeRail`. |
| `frontend/src/components/replay/ReplayWorkspace.tsx` (Hunks 3-10, 12) | 9 | +94 / -54 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | DrawingToolbar toggle removal, indicator order controls, PracticeJournal / DrawingInspector props. |
| `scripts/product-uat.mjs` (Hunk 1) | 1 | +287 / -0 | `AUTHORIZED_P1_INTEGRATION` | **`KEEP_AS_P1`** | Volume Spike E2E browser assertions (registry contract, exact bar identity, no future leakage, INSUFFICIENT_HISTORY, spike true/false, strictly scoped controlled API error via malformed HTTP 200 route.fulfill missing volume.spike, restored VALID via exact-response helper, negative-operation snapshot with pass=true and capturedResponseCount=1 persisted under hardening with zero request failure delta), 1440x1000 screenshot. |
| `scripts/product-uat.mjs` (Hunks 2-10) | 9 | +19 / -9 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Selector resilience (`getByTestId`, `/Cancel\|Hủy/`), magnet select force option, header selector fallback. |
| `frontend/src/components/common/SessionPicker.tsx` | 3 | +8 / -2 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Auto-refetch when selected session missing from query; label formatting `Session #ID`. |
| `AGENTS.md` | 1 | +2 / -0 | `REVIEWER_GOVERNANCE` | **`KEEP_SEPARATE`** | Added mandatory Doraemon targeted freshness check rules (governance, not P1 product code). |
| `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` | 1 | +11 / -0 | `REVIEWER_GOVERNANCE` | **`KEEP_SEPARATE`** | Added Section: "Operational reuse and freshness rule" (governance, not P1 product code). |
| `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` | 1 | +8 / -0 | `REVIEWER_GOVERNANCE` | **`KEEP_SEPARATE`** | Added Section: "P.1a Doraemon/provider targeted freshness gate" (governance, not P1 product code). |
| `backend/app/services/practice_workflow_service.py` | 1 | +30 / -2 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Destructive practice session reset (`reset_practice_session`). |
| `backend/app/services/replay_service.py` | 1 | +14 / -0 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Replay session reset helper (`reset_replay_session`). |
| `backend/app/services/trade_lifecycle_service.py` | 4 | +90 / -41 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Strict T+2 settlement validation and cash requirement enforcement on sell. |
| `backend/app/tests/test_practice_workflow.py` | 3 | +21 / -12 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Tests for practice reset and multi-step bankruptcy stop. |
| `backend/app/tests/test_trade_lifecycle.py` | 6 | +29 / -57 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Tests updated to assert 400 HTTPException on T+1 sell. |
| `frontend/src/App.tsx` | 2 | +3 / -2 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Lazy-loads `DashboardPage` and routes `/` to Dashboard instead of redirecting to `/replay`. |
| `frontend/src/components/layout/Sidebar.tsx` | 2 | +5 / -1 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Navigation links for Dashboard, Backtest, Scanner, and Journal. |
| `frontend/src/components/replay/PracticeRail.tsx` | 4 | +51 / -13 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Dynamic tab bar supporting Trade, Journal, Decisions, and Drawing tabs. |
| `frontend/src/components/replay/TradeControls.tsx` | 7 | +221 / -158 | `OUT_OF_SCOPE_DRIFT` | **`KEEP_SEPARATE`** | Quick SL/TP (2R/3R), Trade Sizing panel, Sync from Drawing, setup/regime/mistake tags. |

---

### 5.3 Exact Phase 1 Include Manifest

The bounded Phase 1 product delivery consists strictly of the following **15 new untracked files** and **3 integration hunks**:

#### A. New Untracked Source & Test Files (15 Files)
1. `backend/app/domain/signals/__init__.py`
2. `backend/app/domain/signals/models.py`
3. `backend/app/domain/signals/registry.py`
4. `backend/app/domain/signals/volume.py`
5. `backend/app/domain/strategy/signal_binding.py`
6. `backend/app/schemas/signal_schema.py`
7. `backend/app/services/signal_service.py`
8. `backend/app/api/signals.py`
9. `backend/app/tests/test_signals.py`
10. `backend/app/tests/test_signal_binding.py`
11. `backend/app/tests/test_signals_api.py`
12. `frontend/src/api/signalsApi.ts`
13. `frontend/src/types/signals.ts`
14. `frontend/src/components/signals/SignalInspector.tsx`
15. `frontend/src/components/signals/__tests__/SignalInspector.test.tsx`

#### B. Integration Hunks in Pre-Existing Files (3 Hunks)
16. `backend/app/main.py`: Router inclusion hunk adding `from app.api import ..., signals` and `app.include_router(signals.router, prefix="/api/signals", tags=["signals"])`.
17. `frontend/src/components/replay/ReplayWorkspace.tsx`: Hunk 1 (import `SignalInspector`), Hunk 2 (empty line formatting), and Hunk 11 (`<SignalInspector sessionId={sessionId} currentIndex={...} timeframe={...} currentTimestamp={...} />` mount in `PracticeRail`).
18. `scripts/product-uat.mjs`: Hunk 1 (lines 568–854, +287 lines: focused Volume Spike browser assertions validating registry, exact bar identity, no future leakage, INSUFFICIENT_HISTORY, spike true, spike false, controlled API error via deterministic route.fulfill missing volume.spike, negative snapshot persistence in hardening with capturedResponseCount=1 and zero request failure delta, restored VALID via exact-response helper, and 1440×1000 screenshot).

---

### 5.4 Separate-Drift Manifest

The following modifications represent pre-existing or parallel Trading Lab, Navigation, Drawing, and Reviewer Governance changes. They are **preserved in the working tree without modification**, **quarantined from Phase 1 review**, and **not claimed as physically cleaned or accepted**:

#### A. Trading Lab & Practice Service Drift
1. `backend/app/services/practice_workflow_service.py`: `reset_practice_session` implementation (+30 / -2).
2. `backend/app/services/replay_service.py`: `reset_replay_session` implementation (+14 / -0).
3. `backend/app/services/trade_lifecycle_service.py`: Strict T+2 sell rejection and cash enforcement (+90 / -41).
4. `backend/app/tests/test_practice_workflow.py`: Unit tests for practice reset (+21 / -12).
5. `backend/app/tests/test_trade_lifecycle.py`: Unit tests for T+2 400 error assertion (+29 / -57).

#### B. UI & Navigation Drift
6. `frontend/src/App.tsx`: Dashboard route `/` and lazy import (+3 / -2).
7. `frontend/src/components/layout/Sidebar.tsx`: Navigation items for Dashboard, Backtest, Scanner, Journal (+5 / -1).
8. `frontend/src/components/replay/PracticeRail.tsx`: Tabs for Journal and Drawing (+51 / -13).
9. `frontend/src/components/replay/TradeControls.tsx`: Quick SL/TP buttons, sizing panel, sync from drawing, setup/regime/mistake selectors (+221 / -158).
10. `frontend/src/components/common/SessionPicker.tsx`: Selected-session auto-refetch effect and label formatting (+8 / -2).
11. `frontend/src/components/replay/ReplayWorkspace.tsx` (Hunks 3–10, 12): Unconditional DrawingToolbar render, indicator order controls, `PracticeJournal` and `DrawingInspector` props (+94 / -54).

#### C. Test Harness Resilience Tweaks
12. `scripts/product-uat.mjs` (Hunks 2–10): Drawing tool selector updates, force option for magnet select, cancel button multilingual regex, header selector fallback (+19 / -9).

#### D. Reviewer Governance & Operational Rules
13. `AGENTS.md`: Doraemon targeted freshness check rules (+2 / -0).
14. `docs/research/DORAEMON_MARKET_DATA_AUDIT.md`: Section on operational reuse and freshness (+11 / -0).
15. `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`: Section P.1a targeted freshness gate (+8 / -0).
16. 15 Reviewer orchestration & prompt documents in `docs/dev-prompts/`, `docs/exec-plans/`, and `docs/reviews/`.

---

## 6. Incremental Phase 1 Integration Deep-Dive

### 6.1 `backend/app/main.py`
- **Baseline State:** Clean at HEAD (`A59EB62CBAC8E033...`).
- **Post-Freeze Diff:**
  ```diff
  @@ -38,10 +38,11 @@
   from app.api import (
       auth,
       backtest,
       candles,
       drawing,
       indicators,
       practice_workflow,
       replay,
       scanner,
  +    signals,
       strategy_lab,
       symbols,
       sync,
       ws_replay,
   )
  ...
   app.include_router(strategy_lab.router, prefix="/api/strategy-lab", tags=["strategy-lab"])
   app.include_router(sync.router, prefix="/api/sync", tags=["sync"])
  +app.include_router(signals.router, prefix="/api/signals", tags=["signals"])
  ```
- **Audit Finding:** Minimal, non-destructive router attachment. Perfectly bounded.

### 6.2 `frontend/src/components/replay/ReplayWorkspace.tsx`
- **Total Post-Freeze Hunks:** 12 hunks (+105 / -54 lines).
- **Approved Phase 1 Signal Inspector Integration:**
  - `Line 23:` `import { SignalInspector } from '../signals/SignalInspector';`
  - `Lines 632-637:` Inside `PracticeRail`:
    ```tsx
    <SignalInspector
      sessionId={sessionId}
      currentIndex={sessionData?.current_index ?? candleCount - 1}
      timeframe={sessionData?.timeframe || '1D'}
      currentTimestamp={currentCandle?.timestamp}
    />
    ```
- **Unrelated Post-Freeze Drift (Trading Lab / Drawing / Journal):**
  - **Drawing Toolbar Visibility:** Removed `isDrawingToolbarOpen` state and header toggle button; `<DrawingToolbar>` rendered unconditionally.
  - **Indicator Order Controls:** Added floating overlay `<div data-testid="indicator-order-controls">` with move up/down buttons for indicator instances.
  - **Chart Legend Overlay:** Wrapped in z-index container `<div>`.
  - **PracticeRail Extensions:** Passed `journal={<PracticeJournal .../>}`, `drawing={<DrawingInspector .../>}`, and `selectedDrawingId`.
- **Reviewer Isolation Strategy:**
  - In a clean patch isolation, only the import and `<SignalInspector>` mount are retained for Phase 1. The remaining UI changes belong to the Trading Lab feature batch.

### 6.3 `scripts/product-uat.mjs`
- **Total Post-Freeze Hunks:** 10 hunks (+306 / -9 lines).
- **Phase 1 Volume Spike UAT Suite (Hunk 1, lines 568–854, +287 / -0 lines):**
  - Verifies `/api/signals/registry` contains valid `volume.spike` v1.0.0 contract.
  - Asserts `<SignalInspector>` visibility.
  - Enforces exact bar identity (`bar_index === observed_current_index`) and strictly asserts no future leakage (`bar_index <= observed_current_index`).
  - Proves `INSUFFICIENT_HISTORY` state with `period=150` on ~60 bars.
  - Proves Spike `true` (`ĐỘT BIẾN`) with multiplier below RVOL and Spike `false` (`BÌNH THƯỜNG`) with multiplier above RVOL.
  - Proves controlled API error via scoped Playwright `route.fulfill` returning malformed HTTP 200 contract (`results: []`, missing `volume.spike`), asserting `signal-error` visible, results/loading absent, `capturedResponseCount === 1`, zero request failure delta, and zero console error suppression.
  - Asserts restored state is strictly `VALID` (explicitly rejecting `COMPLETE`).
  - Retains visual evidence screenshots at 1440×1000: `signal-inspector-1440x1000.png` and `p1-volume-spike-inspector-1440x1000.png`.
- **Hunks 2-10 (Test Harness Resilience Tweaks):**
  - `Line 1170:` `getByTestId('drawing-tool-horizontal')` instead of `getByTitle('Horizontal Line')`.
  - `Line 1654:` `getByRole('button', { name: /Cancel|Hủy/i })` for multilingual resilience.
  - `Line 1746 & 2540:` `selectOption('off', { force: true })` and scroll region resets to prevent Playwright click interception when drawing toolbars overlap.
  - `Line 1909 & 3157:` Header selector fallback `header.replay-header`.
- **Reviewer Isolation Strategy:**
  - Hunk 1 is strictly Phase 1.
  - Hunks 2-10 fix test execution reliability against pre-existing UI elements; they are preserved as separate test-harness stabilization hunks.

---

## 7. Production Database Invariant & Integrity

Read-only inspection was conducted using PowerShell file system primitives and Python hashing utilities without importing application modules or opening an SQLite connection:

```json
{
  "database": {
    "path": "backend/sumi.db",
    "exists": true,
    "size_bytes": 621199360,
    "sha256": "92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64",
    "matches_frozen_baseline": true
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

**Verification:** Zero database writes, schema migrations, or lock files occurred during Phase 1 implementation or verification.

---

## 8. Independent Reproduction Commands

Any independent reviewer can reproduce this exact audit using standard command-line tools:

### 8.1 Fast Git Status & Staged Deletion Verification
```powershell
# Verify branch and commit
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD

# Confirm exactly 159 staged deletions
(git status --porcelain=v1 | Where-Object { $_ -match "^D  " }).Count
(git status --porcelain=v1 | Where-Object { $_ -match "^[MADRC] " -and $_ -notmatch "^D  " }).Count
```

### 8.2 Database Immutability Verification
```powershell
Get-FileHash -Path "backend/sumi.db" -Algorithm SHA256
Test-Path -Path "backend/sumi.db-wal"
Test-Path -Path "backend/sumi.db-shm"
```

### 8.3 Baseline Manifest Verification Script
```python
import hashlib, os

manifest = "scratch/baseline-freeze-20260912-200500/sha256-manifest.txt"
with open(manifest, "r", encoding="utf-8-sig") as f:
    for line in f:
        parts = line.strip().split(maxsplit=3)
        if len(parts) == 4:
            exp_hash, size, cat, rel_path = parts[0], int(parts[1]), parts[2], parts[3]
            norm_path = rel_path.replace("/", os.sep)
            if not os.path.exists(norm_path):
                print(f"MISSING: {rel_path}")
                continue
            with open(norm_path, "rb") as bf:
                cur_hash = hashlib.sha256(bf.read()).hexdigest().upper()
            if cur_hash != exp_hash:
                print(f"CHANGED: {rel_path}")
```

### 8.4 Tracked Baseline Reconstruction & CP437 Decoding
```python
# To reconstruct exact baseline tracked files from git HEAD + unstaged-diff.patch:
with open("scratch/baseline-freeze-20260912-200500/unstaged-diff.patch", "r", encoding="utf-8-sig") as f:
    text = f.read()
# Re-encode through CP437 to resolve Windows PowerShell output encoding:
raw_patch = text.encode("cp437")
with open("scratch/clean-unstaged.patch", "wb") as f:
    f.write(raw_patch)
# Apply to a temporary HEAD checkout directory:
# git apply --check --directory <temp_dir> scratch/clean-unstaged.patch
```

---

## 9. Reviewer Adjudication Recommendations

The repository state is cleanly bifurcated:

1. **Phase 1 Core (`KEEP_AS_P1`):**
   - 15 new untracked Phase 1 files (`backend/app/domain/signals/*`, `backend/app/schemas/signal_schema.py`, `backend/app/services/signal_service.py`, `backend/app/api/signals.py`, backend tests, frontend API/types/SignalInspector, component tests).
   - Router registration in `backend/app/main.py`.
   - SignalInspector import and mount in `frontend/src/components/replay/ReplayWorkspace.tsx`.
   - SignalInspector browser UAT block in `scripts/product-uat.mjs`.

2. **Separate Drift (`KEEP_SEPARATE`):**
   - Trading Lab & practice improvements (`TradeControls.tsx`, `PracticeRail.tsx`, `practice_workflow_service.py`, `replay_service.py`, `trade_lifecycle_service.py`, tests).
   - Dashboard navigation (`App.tsx`, `Sidebar.tsx`, `SessionPicker.tsx`).
   - Non-signal UI hunks in `ReplayWorkspace.tsx`.
   - Test harness resilience hunks in `scripts/product-uat.mjs`.
   - Reviewer governance & operational documentation (`AGENTS.md`, `DORAEMON_MARKET_DATA_AUDIT.md`, `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`, prompt/review docs).

> [!NOTE]
> **Adjudication Path:** Rather than destroying this valuable Trading Lab work (`RESTORE_TO_FROZEN_BASELINE`), it is marked as `KEEP_SEPARATE`. It represents legitimate pre-existing or parallel V3 product work that should be committed or branched under a dedicated "Trading Lab & Practice Improvements" batch once Phase 1 is formally accepted.

---

## 10. Conclusion & Termination Statement

All audit tasks specified in `docs/dev-prompts/P1R_03A_SCOPE_INVENTORY.md`, `docs/dev-prompts/P1R_03B_UAT_SCOPE_CLOSURE.md`, `docs/dev-prompts/P1R_03C_UAT_EVIDENCE_DETERMINISM.md`, and `docs/dev-prompts/P1R_03D_REMOVE_EVIDENCE_SUPPRESSION.md` are complete. No unrelated drift has been cleaned, reset, or modified. Phase 2 has not started.

P1R-03D DONE; PHASE 1 AWAITS REVIEWER SEAL; PHASE 2 NOT STARTED
