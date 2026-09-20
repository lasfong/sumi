# Independent Review Seal: Batch P10-PERF-01 — Profile and Cache Verified Hotspots

**Date**: 2026-09-19  
**Batch ID**: `P10-PERF-01`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P10-PERF-01` delivers the invalidation-safe, bounded domain caching and performance optimization layer for Sumi research hotspots (`SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 523–535; `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` lines 238–239, `NFR-PERF-001`, `NFR-PERF-002`, `TEST-PERF-001`, `TEST-REPRO-001`).

It solves the recalculation hotspot in multi-symbol/multi-phase batch backtests and replay signal scrubbing by introducing an in-memory, thread-safe, LRU-evicting `DomainCache` that is strictly invalidation-safe, encapsulates candle span/checksums, versions, and canonical parameter hashes, and guarantees 100% bit-for-bit deterministic reproducibility.

### Key Architectural Accomplishments

1. **Pure Domain Bounded Cache Engine (`backend/app/domain/engine/cache.py`)**:
   - Implemented thread-safe `DomainCache` with LRU eviction, configurable max entries, and max memory limits.
   - Built deterministic signature functions: `compute_candle_signature` and `compute_strategy_indicators_signature`.
   - Invalidation by symbol, data mutation, indicator parameter change, or signal version bump.
   - Global bypass switch (`use_cache=False` or `cache.disable()`) for deterministic auditability.

2. **Batch Backtest Performance Integration (`backend/app/domain/backtest/batch_runner.py`)**:
   - Extended `build_symbol_feature_cache` to check and populate `DomainCache`.
   - Integrated execution timing metrics (`BatchTimingMetrics`) capturing `total_duration_ms`, `feature_compute_ms`, `simulation_ms`, `cache_hits`, and `cache_misses`.
   - Exposed `timing_metrics` in `BatchBacktestResponse` schema (`backend/app/schemas/backtest_schema.py`) and wired through `BacktestService`.

3. **Replay Signal Scrubbing Caching (`backend/app/services/signal_service.py`)**:
   - Integrated `default_domain_cache` into `calculate_replay_signals` for replay session cursor queries.
   - Cache key strictly encapsulates `session_id`, `current_index`, `signal_name`, `signal_version`, canonical parameter hash, and candle prefix signature.
   - Guaranteed zero future-leak: candles beyond `current_index` are never seen or cached.

4. **Benchmark Harness & Profiling (`backend/scripts/benchmark_research_hotspots.py`)**:
   - Benchmarked 40 tickers × 3 phases (120 backtest runs) and 50 replay scrubbing steps.
   - Achieved **15.91x speedup** on indicator feature computation (from 354.63ms cold down to 22.29ms warm).
   - Achieved 0.14ms for 500 warm signal lookups.
   - Total warm batch backtest duration of 1892.8ms across 120 simulated runs (< 16ms per backtest run).

5. **Frontend Timing Exposure (`frontend/src/components/strategy/MultiPhaseBatchPanel.tsx`)**:
   - Added `timing_metrics` typing to `frontend/src/api/backtestApi.ts`.
   - Exposed performance badge (`data-testid="batch-timing-badge"`) in `MultiPhaseBatchPanel` displaying total duration, feature compute time, simulation time, hits, and misses.
   - Added cache enable/disable checkbox toggle (`data-testid="batch-use-cache-toggle"`).

---

## 2. Invariant & Acceptance Verification

| Acceptance ID / Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `NFR-PERF-001` (Feature Compute Once) | `backend/app/tests/test_performance.py::test_warm_cache_speedup_and_hit_rate` | **PASS** | 40 cache hits, 0 misses on warm run; feature compute time reduced by 94%. |
| `NFR-PERF-002` (Interactive Latency) | `backend/scripts/benchmark_research_hotspots.py` | **PASS** | 120 backtest runs complete in ~1.89s; replay signal lookups in 0.14ms. |
| `TEST-PERF-001` (Speedup & Memory Bounds) | `backend/app/tests/test_performance.py::test_lru_eviction_and_capacity` | **PASS** | Strict LRU eviction respects `max_entries` boundary without leak. |
| `TEST-REPRO-001` (Deterministic Parity) | `backend/app/tests/test_performance.py::test_deterministic_cache_parity` | **PASS** | 100% bit-for-bit numerical parity across uncached, cold, and warm runs (net PnL, trades, returns, cash, equity). |
| Invalidation Safety | `test_cache_invalidation_on_data_mutation`, `test_candle_signature_sensitivity` | **PASS** | Mutated candles or parameters produce signature mismatch and force fresh computation. |
| Zero Future Leak Invariant | `test_signal_service_caching_and_hit_rate` | **PASS** | Caching incorporates `session.current_index`; advancing cursor immediately yields cache miss and recomputes prefix. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Baseline hash strictly matches: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved untouched. |

---

## 3. Automated Test Evidence

1. **Automated Performance Test Suite (`backend/app/tests/test_performance.py`)**:
   ```text
   app/tests/test_performance.py::TestDomainCacheCore::test_lru_eviction_and_capacity PASSED
   app/tests/test_performance.py::TestDomainCacheCore::test_cache_invalidation_symbol PASSED
   app/tests/test_performance.py::TestDomainCacheCore::test_candle_signature_sensitivity PASSED
   app/tests/test_performance.py::TestDomainCacheCore::test_indicator_signature_signature PASSED
   app/tests/test_performance.py::TestDeterministicParityAndPerformance::test_deterministic_cache_parity PASSED
   app/tests/test_performance.py::TestDeterministicParityAndPerformance::test_warm_cache_speedup_and_hit_rate PASSED
   app/tests/test_performance.py::TestDeterministicParityAndPerformance::test_cache_invalidation_on_data_mutation PASSED
   app/tests/test_performance.py::TestDeterministicParityAndPerformance::test_signal_service_caching_and_hit_rate PASSED
   8 passed in 1.42s
   ```

2. **Benchmark Harness Output (`backend/scripts/benchmark_research_hotspots.py`)**:
   ```text
   SUMI RESEARCH HOTSPOTS PERFORMANCE BENCHMARK (P10-PERF-01)
   Executing Cold Batch Backtest: 40 symbols x 3 phases (120 runs)
     Cold Run Duration: 2189.19 ms (Feature: 354.63 ms, Sim: 1732.78 ms, Misses: 40)
   Executing Warm Batch Backtest:
     Warm Run Duration: 1892.78 ms (Feature: 22.29 ms, Sim: 1769.94 ms, Hits: 40)
   >> Feature Cache Speedup: 15.91x
   >> TEST-REPRO-001 Parity Check: [PASS] 100% Bit-for-bit parity confirmed across all 120 runs.
   >> Signal Registry Replay Scrubbing (50 Scrub Steps):
      Cold 50 Scrub Computations: 47.05 ms
      Warm 500 Cached Lookups: 0.14 ms
   ```

3. **Fast Technical Gate (`.\scripts\verify-v2.ps1`)**:
   - Backend pytest: 430 passed, 0 failed.
   - Alembic migration integrity: clean on temp database.
   - Frontend ESLint: clean (0 errors, 0 warnings).
   - Frontend Vitest: 37 test files, 226 passed.
   - Frontend Vite build: production bundle built cleanly in 677ms.
   - Overall: `== Sumi V2 verification complete ==` (Exit Code 0).

4. **Comprehensive Browser E2E UAT (`.\scripts\run-comprehensive-uat.ps1`)**:
   - All 6 domains verified:
     - DOMAIN 1: Replay Engine & Session Lifecycle (PASS)
     - DOMAIN 2: Trading Practice & Real-Time Journal (PASS)
     - DOMAIN 3: Technical Indicators & Multi-Pane Charting (PASS)
     - DOMAIN 4: Drawing System & Geometry (PASS)
     - DOMAIN 5: Strategy Tester & 1-Click Battle (PASS)
     - DOMAIN 6: Navigation, Modals & System Guardrails (PASS)
   - UAT Summary: **31/31 PASSED (100.0%), 0 FAILED**.
   - Zero Console Errors.
   - Zero Database Mutation (`sumi.db` SHA-256 confirmed).

---

## 4. Review Conclusion

Batch `P10-PERF-01` fulfills all acceptance criteria for performance profiling and caching. Invariants for zero future leak, database immutability, and deterministic reproducibility are strictly preserved.

**Independent Review Assessment: ACCEPTED**
