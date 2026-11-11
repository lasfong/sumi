# P10-PERF-01 — Profile and Cache Verified Hotspots

## Outcome
Interactive local research performance without compromising determinism or future-leak invariants. Replay signal calculations and multi-symbol/multi-phase batch backtests benefit from an in-memory, bounded, invalidation-safe cache adapter, with measurable benchmark speedup ($> 2\times$ on warm runs for 30–50 tickers × 3 phases), complete timing metadata in batch responses, and exact bit-for-bit deterministic reproducibility (`TEST-REPRO-001`).

## Context and problem
- **References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 523–535; `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` lines 238–239 (`NFR-PERF-001`, `NFR-PERF-002`, `TEST-PERF-001`, `TEST-REPRO-001`).
- **Audit Findings**: Batch backtesting and replay signal inspection recalculate features when runs are repeated or multiple sweeps evaluate common indicator configurations. While `P4-BATCH-01` introduced intra-batch `SymbolFeatureCache` across phases, cross-run caching, signal series caching, and formal performance profiling across 30–50 tickers have not yet been bounded and benchmarked on the local reference machine.

## In scope
1. **Shared Invalidation-Safe Cache Adapter (`backend/app/domain/engine/cache.py`)**:
   - Explicit domain cache keys encapsulating symbol, timeframe, candle span/checksum, indicator/signal name, version, and sorted parameter hashes.
   - Strict invalidation on any data mutation, parameter change, version bump, or rule change.
   - Bounded memory footprint (LRU with maximum entries and maximum byte limit).
   - Global bypass switch (`use_cache=False` or `cache_enabled=False`) for deterministic auditability.
2. **Batch Backtest Performance Integration (`backend/app/domain/backtest/batch_runner.py`)**:
   - Cross-run feature caching for repeated strategy evaluations or parameter sweeps on identical spanning data.
   - Execution timing instrumentation capturing `total_duration_ms`, `feature_compute_ms`, `simulation_ms`, `cache_hit_count`, and `cache_miss_count`.
   - Thread-safe and cancellation-safe operation.
3. **Replay Signal Calculation Caching (`backend/app/domain/signals/cache.py` or service integration)**:
   - Cache signal calculation series for prefix slices when scrubbing or inspecting signals repeatedly.
4. **Benchmark Harness (`backend/scripts/benchmark_research_hotspots.py`)**:
   - Measures cold vs. warm runtime across 30–50 tickers × 3 phases.
   - Validates speedup factor, memory bounds, and generates machine-readable benchmark reports.
5. **Exact Bit-for-Bit Deterministic Parity Tests (`backend/app/tests/test_performance.py`)**:
   - Rigorous test suite validating `NFR-PERF-001`, `NFR-PERF-002`, `TEST-PERF-001`, `TEST-REPRO-001`.
   - Validates that `use_cache=False` and `use_cache=True` produce identical results down to exact floating point bits.
   - Validates eviction and invalidation on data changes, parameter changes, and signal version changes.
6. **API and UI Timing Exposure**:
   - Expose `timing_metrics` in `BatchBacktestResponse` schema.
   - Display timing badge in `MultiPhaseBatchPanel` (e.g. `⚡ 120ms (Hit: 30, Miss: 0)`).

## Out of scope
- Persisting cache entries to `backend/sumi.db` (cache is purely transient in-memory / optional disk cache file; zero mutation of sumi.db).
- Caching incomplete or non-monotonic candle prefixes.
- Modifying indicator calculation math or signal evaluation logic.

## Invariants
- **Zero Future Leak**: Cache keys strictly respect `observed_index` / date bounds; never cache calculations that see future bars.
- **Authoritative Determinism (`TEST-REPRO-001`)**: A cached calculation must match an uncached fresh calculation with zero difference.
- **Database Immutability**: `backend/sumi.db` SHA-256 hash must remain exactly `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`.
- **Git Invariant**: 159 historically staged deletions remain intact.
- **Bounded Memory**: Cache cannot grow unboundedly; entries must be evicted via LRU when capacity is reached.

## Current architecture
- `BatchBacktestRunner`: Has intra-batch `SymbolFeatureCache` which caches indicator features for the duration of a single `run_batch` call across phases, but discards it immediately when the method returns.
- `SignalService.calculate_replay_signals`: Recalculates all requested signals from scratch on every HTTP POST request, even if called repeatedly at the same candle index.

## Target design
```
+-------------------------------------------------------------+
|                      Client / UI                            |
|             (ReplayPage / StrategyLabPage)                  |
+-------------------------------------------------------------+
                             |
                      HTTP API Request
                             v
+-------------------------------------------------------------+
|                    Application Service                      |
|             (BatchBacktestService / SignalService)          |
+-------------------------------------------------------------+
                             |
            Query Cache by Invalidation-Safe Key
                             v
+-------------------------------------------------------------+
|                   Domain Cache Layer                        |
|  Key = (Symbol, Timeframe, DataHash, FeatureDef, ParamHash) |
|  - Cache HIT: Return precomputed Series / Features          |
|  - Cache MISS: Compute via IndicatorEngine / SignalRegistry |
|  - Invalidation: Evict on Data change / Version change      |
|  - Bounded Memory: LRU with capacity & byte limit           |
+-------------------------------------------------------------+
                             |
               Slices Features Across Phases
                             v
+-------------------------------------------------------------+
|                BacktestExecutionKernel                      |
|         (Independent Capital per Symbol-Phase)              |
+-------------------------------------------------------------+
```

## Milestones
1. **Milestone 1**: Core Cache Engine (`DomainCache` & `CacheKey`) with LRU eviction, memory bounds, and unit tests.
2. **Milestone 2**: Integration into `BatchBacktestRunner` and `SignalService` with timing metadata.
3. **Milestone 3**: Benchmark Harness (`benchmark_research_hotspots.py`) profiling 30–50 tickers × 3 phases on reference machine.
4. **Milestone 4**: Automated Parity and Invalidation Tests (`TEST-REPRO-001`, `TEST-PERF-001`, `NFR-PERF-001/002`).
5. **Milestone 5**: Frontend timing metadata exposure and full technical gates verification.

## Acceptance mapping
| Acceptance ID | Implementation Evidence | Test/UAT Evidence |
|---|---|---|
| `NFR-PERF-001` | Feature computation once per symbol across spanning range | `test_compute_once_feature_caching`, `test_cache_hit_rate` |
| `NFR-PERF-002` | Interactive local research on 30–50 tickers × 3 phases | `backend/scripts/benchmark_research_hotspots.py` benchmark report |
| `TEST-PERF-001` | Cold vs. warm speedup measurement and memory bounds | `test_cold_vs_warm_speedup`, `test_memory_bounds_lru` |
| `TEST-REPRO-001` | Bit-for-bit deterministic parity (cache enabled vs disabled) | `test_deterministic_cache_parity` |
| Invalidation Safety | Invalidate on candle, parameter, or version change | `test_cache_invalidation_on_data_change`, `test_cache_invalidation_on_param_change` |

## Verification commands
```powershell
# 1. Focused Performance & Caching Tests
pytest backend/app/tests/test_performance.py -v

# 2. Run Benchmark Harness
python backend/scripts/benchmark_research_hotspots.py

# 3. Fast Technical Gate
.\scripts\verify-v2.ps1

# 4. Comprehensive Browser UAT
.\scripts\run-comprehensive-uat.ps1

# 5. DB Hash Integrity
Get-FileHash backend/sumi.db -Algorithm SHA256
```

## Rollback and compatibility
- In-memory cache can be disabled at any time via `DomainCache.disable()` or passing `use_cache=False`.
- No database migrations or schema alterations to persistent tables.
- If reverted, standard batch runner behavior continues without caching.

## Risks and mitigations
- **Risk**: Incomplete cache key causing stale data when indicators or candles change.
  - **Mitigation**: Hash candle timestamps, length, indicator version, and canonical sorted parameter dictionary.
- **Risk**: Memory leakage on large batch sweeps.
  - **Mitigation**: Bounded LRU cache with configurable maximum entries and explicit eviction.

## Progress log
- 2026-09-19: ExecPlan drafted and batch `P10-PERF-01` transitioned to running.
- 2026-09-19: Implemented `DomainCache` in `backend/app/domain/engine/cache.py` with LRU eviction and deterministic signatures.
- 2026-09-19: Integrated caching and timing instrumentation into `BatchBacktestRunner` and `SignalService`.
- 2026-09-19: Created and verified test suite `backend/app/tests/test_performance.py` (8/8 tests passing).
- 2026-09-19: Implemented and executed benchmark harness `backend/scripts/benchmark_research_hotspots.py` (15.91x feature compute speedup, 100% parity).
- 2026-09-19: Integrated timing badge and cache toggle into `MultiPhaseBatchPanel.tsx`.
- 2026-09-19: Passed technical gate `verify-v2.ps1` and comprehensive browser UAT `run-comprehensive-uat.ps1` (31/31 passed, 0 console errors, 0 db mutation).
- 2026-09-19: Independent review seal issued in `docs/reviews/P10_PERF_01_REVIEW.md` (ACCEPTED).

## Decision log
- Decision: Use thread-safe in-memory LRU cache with pure domain key generation rather than external dependency like Redis.
- Rationale: Preserves local-first zero-telemetry architecture without introducing external daemon requirements.

