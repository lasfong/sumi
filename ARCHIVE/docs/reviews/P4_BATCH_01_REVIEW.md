# Independent Review Seal: Batch P4-BATCH-01 — Compute-Once Phase Runner

**Date**: 2026-09-17  
**Batch ID**: `P4-BATCH-01`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P4-BATCH-01` delivers multi-symbol, multi-phase backtest orchestration with compute-once feature caching and independent capital isolation, strictly adhering to the roadmap invariants and Vietnamese market rules. Key deliverables include:

1. **Domain Orchestrator** (`backend/app/domain/backtest/batch_runner.py`):
   - `PhaseDefinition`: validated evaluation time windows with strict date ordering (`start_date < end_date`) and unique phase identification.
   - `SymbolFeatureCache`: precomputes indicators and features exactly once per symbol over a spanning window (all phases plus 90-day warmup).
   - `BatchBacktestRunner`: slices precomputed indicator series without recomputing (`TEST-PERF-001`, `NFR-PERF-001`), executes Next-Event daily simulations with fresh independent capital per phase (`FR-CORE-009`), and enforces phase-end holding integrity (`TEST-BT-005`).
2. **Schema & API Contracts** (`backend/app/schemas/backtest_schema.py`, `backend/app/api/backtest.py`):
   - `PhaseDefinitionRequest`, `BatchBacktestRequest`, `BatchPhaseResultResponse`, `BatchBacktestResponse`.
   - `POST /api/backtest/batch/run`: dedicated batch evaluation endpoint.
   - `POST /api/backtest/run`: transparent routing to batch runner when `phases` parameter is supplied.
3. **Service Layer Integration** (`backend/app/services/backtest_service.py`):
   - `run_batch_backtest`: database candle loading adapter and batch runner orchestration with zero DB writes during simulation.
4. **Model Enhancements** (`backend/app/domain/backtest/models.py`):
   - Added backwards-compatible convenience properties (`final_cash`, `final_equity`, `final_position`, `trades`, `warnings`) on `BacktestKernelResult`.

---

## 2. Invariant & Performance Verification

| Acceptance Invariant | Verification Command / Target | Result | Evidence |
|---|---|---|---|
| `NFR-PERF-001` / `TEST-PERF-001` (Compute-once feature caching) | `test_batch_runner.py::test_compute_once_feature_caching` | **PASS** | For $S=2$ symbols and $P=3$ phases, indicator computation runs exactly 2 times ($S$), not 6 ($S \times P$). `feature_compute_count == 2`, `simulation_run_count == 6`. |
| `FR-CORE-009` (Independent capital isolation) | `test_batch_runner.py::test_independent_capital_isolation` | **PASS** | Each phase simulation begins with independent `initial_cash_per_run`. Profits and losses never bleed across phase boundaries. |
| `TEST-BT-005` (Phase-end holding integrity) | `test_batch_runner.py::test_phase_end_holding_integrity` | **PASS** | Positions open at phase end remain open and are marked to market in `final_equity`; strictly excluded from closed trade metrics (`total_trades == 0`). |
| Single-Symbol Parity | `test_batch_runner.py::test_single_symbol_batch_parity` | **PASS** | Direct kernel execution and batch sliced execution produce bit-for-bit identical final cash, trade counts, and open positions. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Hash matches baseline: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Inherited Work Preservation | `git status -s` | **PASS** | Exactly 159 historical staged deletions preserved. |

---

## 3. Test & Gate Execution Results

### 3.1 Focused Batch Runner Test Suite
```text
pytest backend/app/tests/test_batch_runner.py -v
Result: 10 passed in 0.60s (100%)
```

### 3.2 Full Backtest & Market Rules Suite
```text
pytest backend/app/tests/test_batch_runner.py backend/app/tests/test_backtest_kernel.py backend/app/tests/test_backtest.py backend/app/tests/test_market_rules.py -v
Result: 51 passed in 1.09s (100%)
```

### 3.3 Fast Technical Gate (`verify-v2.ps1`)
- **Backend Tests**: 285/285 passed (pytest)
- **Database Migrations**: Alembic upgrade clean on isolated test DB
- **Frontend Tests**: 210/210 passed across 32 test files (Vitest)
- **Frontend Build**: `tsc -b && vite build` passed without error
- **Exit Code**: 0

### 3.4 Comprehensive Browser UAT (`run-comprehensive-uat.ps1`)
- **Scenarios Executed**: 31/31 passed (100.0%)
- **Console Errors**: 0
- **Report Location**: `test-results/comprehensive-uat/report.json`
- **Exit Code**: 0

---

## 4. Acceptance Certification

Batch `P4-BATCH-01` satisfies all criteria set out in `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` and `docs/exec-plans/P4_BATCH_01_COMPUTE_ONCE_PHASE_RUNNER.md`. All required gates pass with zero regressions.

**Seal**: Verified and Accepted by Independent Reviewer.
