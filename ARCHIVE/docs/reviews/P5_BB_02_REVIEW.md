# Independent Review Seal: Batch P5-BB-02 — Symbol BB with OHLCV_PROXY

**Date**: 2026-09-18  
**Batch ID**: `P5-BB-02`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P5-BB-02` delivers the production calculation engine and API infrastructure for the Symbol Money Flow Blackbox (BB) under the audited `OHLCV_PROXY` methodology. It implements the exact mathematical formulation specified in the authoritative Blackbox Handoff and Master Spec, consuming canonical bars via `MarketDataPort`, enforcing strict Acceptance Test invariants (`AT01`, `AT05–AT08`, `AT13`, `AT15`, `AT16`, `AT18`, `AT20`), and providing complete traceability without look-ahead bias or false order-flow claims.

Key deliverables include:

1. **Pure BB Proxy Calculator** (`backend/app/domain/bb/calculator.py`):
   - `ProxyBBCalculator`: pure, deterministic multi-horizon calculation engine.
   - Authoritative True Range ($TH_t = \max(H_t, C_{t-1})$, $TL_t = \min(L_t, C_{t-1})$) incorporating overnight gaps (`AT15`).
   - Price pressure calculation: $P_t = \frac{2 C_t - TH_t - TL_t}{TH_t - TL_t}$, clamped to $[-1.0, 1.0]$. Zero-range flat candles ($TH == TL$) deterministically produce $P_t = 0.0$ without division errors (`AT01`, `AT15`).
   - Turnover weighting: $SP_t = P_t \times V_t$.
   - Multi-horizon rolling evaluation ($H \in \{3, 5, 10, 20, 50, 200\}$):
     - Warm-up policy: initial $t + 1 < H$ sessions evaluate to `None` with `is_warmup=True` (`AT08`).
     - Rolling windows: slices $t-H+1 \dots t$ update continuously per session, not in discrete blocks (`AT06`).
     - Zero activity guard: if rolling turnover sum $= 0$, score is `None`, never silently defaulted to 50.0 (`AT05`).
     - Bounded outputs: $\text{oib\_raw} \in [-1.0, 1.0]$ and $BB_H \in [0.0, 100.0]$ (`AT01`).
     - Causal state evaluation: direction (`RISING`, `FALLING`, `FLAT`) and regime (`POSITIVE`, `NEGATIVE`, `NEUTRAL`) with consecutive run lengths depend strictly on current and prior valid observations (`AT18`).
     - Prefix invariance: future records cannot alter past calculations (`AT07`).
     - Window metadata aggregation: conservative `DataQuality` and traceable `ValueSource` (`AT16`).

2. **Domain Contracts Extension** (`backend/app/domain/bb/contracts.py`, `__init__.py`):
   - Added `BBDirection`, `BBRegime`, `BBSymbolHorizonPoint`, `BBSymbolPoint`, and `BBSymbolSeriesResult`.

3. **Replay Chart Adapter** (`backend/app/domain/bb/chart_adapter.py`):
   - `BBChartAdapter.to_chart_series`: formats `BBSymbolSeriesResult` into lightweight, frontend-ready time-series records.

4. **Service & Port Integration** (`backend/app/services/bb_service.py`, `backend/app/domain/data/`):
   - `MoneyFlowBlackboxService`: orchestrates data retrieval through `MarketDataPort` (`SumiCandleAdapter`, `FixtureMarketDataAdapter`, `DoraemonMarketDataAdapter`).
   - Extended `get_canonical_bars` across all adapters to support optional date filtering and lookback `limit`.

5. **API Schemas & Endpoints** (`backend/app/schemas/bb_schema.py`, `backend/app/api/bb.py`, `backend/app/main.py`):
   - `GET /api/bb/horizons`: returns registry of supported BB horizons.
   - `GET /api/bb/symbol/{symbol}`: fetches and calculates BB series or chart data for a symbol.
   - `POST /api/bb/calculate`: calculates on-demand from request payload.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `AT01 Bounds` | `test_bb_calculator.py::test_bb_bounds_and_oib_clamping` | **PASS** | Every valid BB is in $[0, 100]$; every raw flow in $[-1, +1]$; single-bar pressure clamped to $[-1, +1]$. |
| `AT05 Zero activity` | `test_bb_calculator.py::test_bb_zero_activity_denominator_null` | **PASS** | If rolling volume denominator $= 0 \implies BB = \text{None}$, never silently 50. |
| `AT06 Rolling not block` | `test_bb_calculator.py::test_bb_rolling_window_not_block` | **PASS** | Day 7 for T05 strictly uses sessions 3..7, independent of session 1. |
| `AT07 Future invariance` | `test_bb_calculator.py::test_bb_prefix_future_invariance` | **PASS** | Appending future records does not alter prior BB points or state. |
| `AT08 Warm-up` | `test_bb_calculator.py::test_bb_warmup_policy` | **PASS** | T_H is null until H valid sessions (T200 null across first 200 bars). |
| `AT13 Method boundary` | `test_bb_calculator.py::test_bb_method_and_version_traceability` | **PASS** | Output strictly reports `flow_method=OHLCV_PROXY` and `methodology_version="bb_v1_ohlcv_proxy"`. |
| `AT15 Proxy formula` | `test_bb_calculator.py::test_bb_true_range_and_flat_candle_handling` | **PASS** | True range uses previous close; zero-range flat candle yields 0 pressure without division error. |
| `AT16 Value source` | `test_bb_calculator.py::test_bb_value_source_traceability` | **PASS** | Value source correctly traces actual matched value vs typical price estimate. |
| `AT18 Turn causality` | `test_bb_calculator.py::test_bb_direction_and_regime_causality` | **PASS** | Direction and regime depend only on current session $t$ and previous $t-1$. |
| `AT20 Reproducibility` | `test_bb_calculator.py::test_bb_reproducibility` | **PASS** | Identical inputs produce bit-for-bit identical results. |
| `FR-CORE-012` | `test_bb_calculator.py::test_bb_service_with_fixture_port` | **PASS** | BB Service calculates correctly from any `MarketDataPort` adapter. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Baseline hash preserved: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved intact. |

---

## 3. Test & Gate Execution Results

### 3.1 Focused Blackbox Calculator Suite
```text
============================= test session starts =============================
app/tests/test_bb_calculator.py::test_bb_bounds_and_oib_clamping PASSED  [  7%]
app/tests/test_bb_calculator.py::test_bb_zero_activity_denominator_null PASSED [ 15%]
app/tests/test_bb_calculator.py::test_bb_rolling_window_not_block PASSED [ 23%]
app/tests/test_bb_calculator.py::test_bb_prefix_future_invariance PASSED [ 30%]
app/tests/test_bb_calculator.py::test_bb_warmup_policy PASSED            [ 38%]
app/tests/test_bb_calculator.py::test_bb_method_and_version_traceability PASSED [ 46%]
app/tests/test_bb_calculator.py::test_bb_true_range_and_flat_candle_handling PASSED [ 53%]
app/tests/test_bb_calculator.py::test_bb_value_source_traceability PASSED [ 61%]
app/tests/test_bb_calculator.py::test_bb_direction_and_regime_causality PASSED [ 69%]
app/tests/test_bb_calculator.py::test_bb_reproducibility PASSED          [ 76%]
app/tests/test_bb_calculator.py::test_bb_chart_adapter PASSED            [ 84%]
app/tests/test_bb_calculator.py::test_bb_service_with_fixture_port PASSED [ 92%]
app/tests/test_bb_calculator.py::test_bb_api_endpoints PASSED            [100%]
======================== 13 passed, 1 warning in 0.10s ========================
```

### 3.2 Fast Technical Gate (`verify-v2.ps1`)
- **Backend Tests**: 319 passed (0 failures).
- **Alembic Upgrade**: Successfully applied migrations on temporary SQLite test DB.
- **Frontend Lint**: Clean (`eslint .`).
- **Frontend Tests**: 32 test files passed, 210 tests passed.
- **Frontend Build**: TypeScript checking and Vite production build passed.

### 3.3 Comprehensive Browser UAT (`run-comprehensive-uat.ps1`)
- **Result**: 31/31 passed (100.0%, 0 failed).
- **Database Integrity Guardrail (`TC-SYS-05`)**: PASSED. SHA-256 verified match.
- **Zero Console Errors (`TC-SYS-04`)**: PASSED.

---

## 4. Architectural Boundaries & R&D Guardrails

- **Pure Domain Math**: `ProxyBBCalculator` is completely decoupled from database models, web frameworks, and external dependencies.
- **Strict Method Boundary**: Strictly operates under `FlowMethod.OHLCV_PROXY`. No unvalidated claims of "actual money flow" are produced.
- **Empirical Threshold Guardrail**: No 20/30/70/80 trading thresholds are hardcoded as production rules, maintaining clean handoff to `P6-MEASURE-01`.

---

## 5. Audit Review Seal

All acceptance criteria, invariants, and quality gates for `P5-BB-02` are satisfied in full without regressions or database mutations.

**SEAL STATUS**: **ACCEPTED**  
**RECOMMENDED NEXT TASK**: `P6-MEASURE-01` (BB behavior measurement study on real audited sample, not P&L optimization).
