# P4-BATCH-01 — Compute-Once Phase Runner

## Outcome
Sumi provides a multi-symbol, multi-phase backtest orchestration engine with strictly independent capital per simulation (`FR-CORE-009`), compute-once indicator and signal feature caching (`NFR-PERF-001`, `TEST-PERF-001`), approved phase-end policies (`TEST-BT-005`), and exact single-symbol execution parity.

## Context and problem
- Canonical source: `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` (Phase 4, `P4-BATCH-01`).
- Mathematical specification: `docs/research/SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 2A, 22/I, Appendix J (`TEST-PERF-001`, `TEST-BT-005`, `FR-CORE-008`, `FR-CORE-009`).
- Previously, multi-symbol backtests ran only over a single monolithic date range and re-read/re-computed data from scratch for every execution. Simulating multiple market regimes (e.g. Bull 2020-2021, Bear 2022, Recovery 2023-2024) across a universe required repeated, redundant indicator calculations and lacked structured phase definitions.

## In scope
1. **Phase Models & Validation** (`backend/app/domain/backtest/batch_runner.py`):
   - `PhaseDefinition` with `name`, `start_date`, `end_date`, `description`.
   - Validation against empty phase sets, duplicate names, and inverted date ranges (`start_date >= end_date`).
2. **Compute-Once Feature Cache** (`SymbolFeatureCache`):
   - Computes spanning candle DataFrame and indicator series once per symbol with necessary warmup before the earliest phase.
   - Slices the precomputed feature cache for each phase run without recomputing indicators (`TEST-PERF-001`).
3. **Independent Capital (`FR-CORE-009`)**:
   - Each `(symbol, phase)` starts with a fresh independent cash balance (`initial_cash`). Zero shared capital across symbols or phases.
4. **Phase-End Holding Integrity (`TEST-BT-005`)**:
   - Open or unsellable positions at phase cutoff are marked to market in the equity curve, but are not counted as closed trades.
5. **Single-Symbol Parity**:
   - Identical trades, orders, and equity whether a `(symbol, phase)` is executed individually or inside a multi-symbol batch.
6. **API & Service Integration**:
   - Schemas in `backend/app/schemas/backtest_schema.py`.
   - Service orchestration in `backend/app/services/backtest_service.py`.
   - Dedicated endpoint `POST /api/backtest/batch/run` and backward-compatible `POST /api/backtest/run`.

## Out of scope
- P4-MET-02 (Exact nine benchmark metrics, CSV export, and table presentation formatting).
- Persistent multi-run DB schema migrations (reuses existing in-memory / JSON run artifacts).
- Frontend UI phase editor components (delivered with P4-MET-02 / P9-UI-01).

## Invariants
- **No Future Leakage**: Warmup and feature computation over the spanning range are strictly causal.
- **Compute-Once Verification (`TEST-PERF-001`)**: Indicator computation count must equal the number of distinct symbols, not $\text{symbols} \times \text{phases}$.
- **Zero Database Mutation**: Never mutate or connect to `backend/sumi.db` in automated tests.
- **Fail-Safe Independence**: A failure in one symbol or phase (e.g. missing data) must not crash other valid symbols or phases; returns status `"partial"` with explicit error details.

## Current architecture
- `backend/app/services/backtest_service.py` runs single-symbol backtests and basic multi-symbol backtests on a single date range.
- `BacktestExecutionKernel` executes Next-Event daily simulations in memory.
- No phase abstraction or spanning feature cache currently exists.

## Target design
```mermaid
flowchart TD
    REQ[BatchBacktestRequest: symbols[], phases[], strategy, initial_cash] --> BR[BatchRunner]
    BR --> SPAN[Determine Spanning Date Range + Warmup per Symbol]
    SPAN --> LOAD[Load Spanning Candles once per symbol]
    LOAD --> CACHE[SymbolFeatureCache: Compute Indicators ONCE per symbol]
    CACHE --> SLICE1[Slice Symbol A, Phase 1]
    CACHE --> SLICE2[Slice Symbol A, Phase 2]
    CACHE --> SLICE3[Slice Symbol B, Phase 1]
    CACHE --> SLICE4[Slice Symbol B, Phase 2]
    SLICE1 --> K1[Kernel Run: Independent Capital]
    SLICE2 --> K2[Kernel Run: Independent Capital]
    SLICE3 --> K3[Kernel Run: Independent Capital]
    SLICE4 --> K4[Kernel Run: Independent Capital]
    K1 & K2 & K3 & K4 --> AGG[BatchBacktestResult: Per-phase outcomes + Perf Counters]
```

## Milestones
1. **Milestone 1**: Implement `backend/app/domain/backtest/batch_runner.py` with `PhaseDefinition`, `SymbolFeatureCache`, compute-once orchestration, and independent capital. [DONE]
2. **Milestone 2**: Implement schemas in `backend/app/schemas/backtest_schema.py` and extend `backend/app/services/backtest_service.py` and `backend/app/api/backtest.py`. [DONE]
3. **Milestone 3**: Implement comprehensive unit test suite `backend/app/tests/test_batch_runner.py` proving `TEST-PERF-001`, `FR-CORE-009`, `TEST-BT-005`, and single-symbol parity. [DONE]
4. **Milestone 4**: Run full verification gates (`verify-v2.ps1` and `run-comprehensive-uat.ps1`). [DONE]
5. **Milestone 5**: Obtain independent review, seal `docs/reviews/P4_BATCH_01_REVIEW.md`, and advance `docs/dev-program/STATE.json`. [DONE]

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence | Status |
| --- | --- | --- | --- |
| `FR-CORE-008` | `batch_runner.py::run_batch` | `test_batch_runner.py::test_compute_once_feature_caching` | PASS |
| `FR-CORE-009` | `batch_runner.py` independent cash per run | `test_batch_runner.py::test_independent_capital_isolation` | PASS |
| `NFR-PERF-001` / `TEST-PERF-001` | `SymbolFeatureCache` compute once | `test_batch_runner.py::test_compute_once_feature_caching` | PASS |
| `TEST-BT-005` | `execution.py` & `batch_runner.py` phase-end holding | `test_batch_runner.py::test_phase_end_holding_integrity` | PASS |
| Single-Symbol Parity | `batch_runner.py` slicing | `test_batch_runner.py::test_single_symbol_batch_parity` | PASS |

## Verification commands
```powershell
# 1. Focused batch runner unit tests
pytest backend/app/tests/test_batch_runner.py backend/app/tests/test_backtest_kernel.py -v

# 2. Fast technical gate
.\scripts\verify-v2.ps1

# 3. Comprehensive browser UAT
.\scripts\run-comprehensive-uat.ps1
```

## Rollback and compatibility
If needed, `batch_runner.py` can be removed, and `backtest_service.py` reverted without affecting existing single-run backtests or database models.

## Risks and mitigations
- **Risk**: Memory consumption when caching large universes.
  - **Mitigation**: Precompute and process symbols sequentially or in bounded batches; release candle DataFrames after slicing.
- **Risk**: Phase boundary slicing errors (off-by-one in index or timestamp).
  - **Mitigation**: Slice by strict timestamp filtering (`start_at(phase.start_date)` to `end_before(phase.end_date)`), and re-index sliced DataFrames from 0 for the Next-Event kernel.

## Progress log
- 2026-09-17: ExecPlan created. Implementation started.
- 2026-09-17: Milestone 1 & 2 completed (`batch_runner.py`, `backtest_schema.py`, `backtest_service.py`, `backtest.py`).
- 2026-09-17: Milestone 3 completed (`test_batch_runner.py` 10/10 passed).
- 2026-09-17: Milestone 4 completed (`verify-v2.ps1` 285/285 backend passed, 210/210 frontend passed; comprehensive UAT 31/31 passed 100%).
- 2026-09-17: Milestone 5 completed (`P4_BATCH_01_REVIEW.md` sealed). Batch accepted.
