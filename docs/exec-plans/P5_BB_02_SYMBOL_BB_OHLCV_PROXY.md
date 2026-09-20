# P5-BB-02 — Symbol BB with OHLCV_PROXY

## Outcome
A high-integrity, deterministic Symbol Money Flow Blackbox (BB) calculation engine implementing the v3 `OHLCV_PROXY` formula across standard horizons (`T03`, `T05`, `T10`, `T20`, `T50`, `T200`), consuming canonical bars from `MarketDataPort`, providing complete traceability (value source, data quality, methodology version, raw numerator and denominator, warmup handling, rolling direction, and regime run lengths), fully compliant with applicable BB Acceptance Tests (`AT01`, `AT05-AT08`, `AT13`, `AT15`, `AT16`, `AT18`, `AT20`), and exposing REST APIs and Replay chart adapters without violating future-invariance or zero database mutation invariants.

## Context and problem
- **Addressed requirements:** `FR-CORE-012`, `BBI-SIG-001`, `BBI-SIG-003`, `TEST-BB-001`, `TEST-BB-002`, `AT01`, `AT05`, `AT06`, `AT07`, `AT08`, `AT13`, `AT15`, `AT16`, `AT18`, `AT20`.
- **Context:** `P5-DATA-01` delivered `MarketDataPort`, `CanonicalBar`, and BB contracts foundation.
- **Problem:** Traders and technical analysis practitioners need an institutional-grade, value-weighted technical flow pressure proxy that operates cleanly under Vietnam daily bar constraints without making false claims of "actual buy/sell order flow" or silently imputing missing values.

## In scope
1. **Pure BB Proxy Calculator** (`backend/app/domain/bb/calculator.py`):
   - Exact mathematical implementation of `OHLCV_PROXY` formula:
     - True High: $TH_t = \max(H_t, C_{t-1})$, for $t=0: H_0$
     - True Low: $TL_t = \min(L_t, C_{t-1})$, for $t=0: L_0$
     - Pressure: $P_t = \frac{2 C_t - TH_t - TL_t}{TH_t - TL_t}$ if $TH_t > TL_t$ else $0.0$.
     - Clamped to $[-1.0, 1.0]$.
     - Value: $V_t = \text{trading\_value}_t$, Signed Pressure: $SP_t = P_t \times V_t$.
     - Rolling Horizon $H \in \{3, 5, 10, 20, 50, 200\}$:
       - Warmup: if $t + 1 < H \implies \text{None}$ (AT08).
       - Rolling sums: $\Sigma_H SP$, $\Sigma_H V$.
       - Denominator zero handling: if $\Sigma_H V == 0 \implies \text{None}$ (AT05).
       - Raw Flow: $\text{oib\_raw} = \frac{\Sigma_H SP}{\Sigma_H V} \in [-1.0, 1.0]$.
       - BB Score: $BB_H = 50 \times (1 + \text{oib\_raw}) \in [0.0, 100.0]$ (AT01).
       - Rolling direction: `RISING` ($BB_t > BB_{t-1}$), `FALLING` ($BB_t < BB_{t-1}$), `FLAT` ($BB_t == BB_{t-1}$).
       - Regime: `POSITIVE` ($BB_t > 50$), `NEGATIVE` ($BB_t < 50$), `NEUTRAL` ($BB_t == 50$).
       - `regime_run_length`: count of consecutive bars in active regime.
       - Quality & ValueSource aggregation across window (traceable metadata).
2. **BB Series Models & Manifests** (`backend/app/domain/bb/contracts.py`):
   - `BBSymbolPoint`: point container across all requested horizons for a given calendar date.
   - `BBSymbolSeriesResult`: multi-horizon result with metadata (`symbol`, `methodology_version="bb_v1_ohlcv_proxy"`, `flow_method=FlowMethod.OHLCV_PROXY`, `as_of`, `points`, `coverage_ratio`).
3. **BB Domain Service & Integration** (`backend/app/services/bb_service.py`):
   - `MoneyFlowBlackboxService`: encapsulates calculation execution with caching support (in-memory hash-keyed cache).
4. **BB API Schemas & Routes** (`backend/app/schemas/bb_schema.py`, `backend/app/api/bb.py`):
   - `GET /api/bb/symbol/{symbol}`: returns BB multi-horizon series for a symbol.
   - `POST /api/bb/calculate`: calculates on-demand from provided request / bars.
   - Mount router in `backend/app/main.py`.
5. **Replay Chart Adapter** (`backend/app/domain/bb/chart_adapter.py`):
   - Lightweight adapter formatting BB multi-horizon points for chart consumption.
6. **Comprehensive Test Suite** (`backend/app/tests/test_bb_calculator.py`):
   - Unit and integration tests covering AT01, AT05, AT06, AT07, AT08, AT13, AT15, AT16, AT18, AT20, prefix invariance, zero-range handling, clamp behavior, and API routes.

## Out of scope
- True Flow / order-level buy-sell classifier (`CLASSIFIED_ORDER_FLOW`, `EXECUTED_ORDER_FLOW`).
- Whole-market breadth and market aggregation (`P7-UNI-01`, `P7-MKT-02`).
- Hardcoded trading strategy signals on 20/30/70/80 thresholds (deferred until empirical study `P6-MEASURE-01`).
- Mutation of `backend/sumi.db`.

## Invariants
- **Future invariance (AT07):** Appending future bars never changes historical BB scores.
- **No silent imputation (NFR-DQ-001):** Missing values tagged explicitly; zero volume denominator yields `None`, never 50.0.
- **Zero database mutation:** Tests and UAT use in-memory SQLite (`:memory:`) or temporary databases.
- **Git hygiene:** 159 historically staged deletions preserved.

## Milestones
1. [x] **Milestone 1**: Implement pure BB calculator in `backend/app/domain/bb/calculator.py` with multi-horizon rolling math, AT01/05/06/07/08/15/16/18/20 compliance.
2. [x] **Milestone 2**: Extend `backend/app/domain/bb/contracts.py` with series models (`BBSymbolPoint`, `BBSymbolSeriesResult`), chart adapter in `backend/app/domain/bb/chart_adapter.py`, and service in `backend/app/services/bb_service.py`.
3. [x] **Milestone 3**: Implement API schemas in `backend/app/schemas/bb_schema.py` and endpoints in `backend/app/api/bb.py`, registered in `backend/app/main.py`.
4. [x] **Milestone 4**: Implement exhaustive unit & integration test suite in `backend/app/tests/test_bb_calculator.py`.
5. [x] **Milestone 5**: Execute fast technical gate (`verify-v2.ps1`) and comprehensive browser UAT (`run-comprehensive-uat.ps1`).
6. [x] **Milestone 6**: Independent review seal, ExecPlan completion, and `STATE.json` update.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `AT01 Bounds` | `calculator.py::ProxyBBCalculator` clamping | `test_bb_calculator.py::test_bb_bounds_and_oib_clamping` |
| `AT05 Zero activity` | `calculator.py::ProxyBBCalculator` zero denominator check | `test_bb_calculator.py::test_bb_zero_activity_denominator_null` |
| `AT06 Rolling not block` | Rolling window slice `t-H+1..t` | `test_bb_calculator.py::test_bb_rolling_window_not_block` |
| `AT07 Future invariance` | Prefix slice invariance test | `test_bb_calculator.py::test_bb_prefix_future_invariance` |
| `AT08 Warm-up` | `is_warmup` flag and `None` value when $t+1 < H$ | `test_bb_calculator.py::test_bb_warmup_policy` |
| `AT13 Method boundary` | Strict `FlowMethod.OHLCV_PROXY` tag | `test_bb_calculator.py::test_bb_method_and_version_traceability` |
| `AT15 Proxy formula` | True-range pressure formula & zero-range handling | `test_bb_calculator.py::test_bb_true_range_and_flat_candle_handling` |
| `AT16 Value source` | Traceable `ValueSource` aggregation | `test_bb_calculator.py::test_bb_value_source_traceability` |
| `AT18 Turn causality` | Direction/Regime depends only on $t$ and $t-1$ | `test_bb_calculator.py::test_bb_direction_and_regime_causality` |
| `AT20 Reproducibility` | Deterministic outputs across runs | `test_bb_calculator.py::test_bb_reproducibility` |
| `FR-CORE-012` | BB Service & Port integration | `test_bb_calculator.py::test_bb_service_with_fixture_port` |
| `BBI-SIG-001` | Horizon outputs T03/T05/T10/T20/T50/T200 | `test_bb_calculator.py::test_bb_warmup_policy`, `test_bb_bounds_and_oib_clamping` |
| `BBI-SIG-003` | Permitted method / quality filter | `test_bb_calculator.py::test_bb_api_endpoints` |

## Verification commands
```powershell
# 1. Focused BB calculator test suite
.\.venv\Scripts\python.exe -m pytest app/tests/test_bb_calculator.py -v

# 2. Fast technical gate
.\scripts\verify-v2.ps1

# 3. Comprehensive browser UAT
.\scripts\run-comprehensive-uat.ps1
```

## Rollback and compatibility
`P5-BB-02` adds pure domain calculator and new `/api/bb` routes. No existing tables or schemas are modified. Rollback simply removes the new files.

## Risks and mitigations
- **Risk:** Missing volume causing divide-by-zero.
  - **Mitigation:** Strict `if denominator == 0.0 -> None` guardrail per AT05.
- **Risk:** Flat candles where High == Low == Close causing zero range.
  - **Mitigation:** Zero range yields pressure = 0.0 per AT15.
- **Risk:** Large data series calculation performance.
  - **Mitigation:** Rolling sums calculated in $O(N)$ using sliding accumulator or NumPy.

## Progress log
- 2026-09-18: ExecPlan created for P5-BB-02. Target architecture and AT mapping established.
- 2026-09-18: Implemented `backend/app/domain/bb/calculator.py` with pure `ProxyBBCalculator` executing authoritative True Range, pressure, signed value, rolling sums, warmup nulls, and regime/direction logic.
- 2026-09-18: Extended `backend/app/domain/bb/contracts.py` with `BBDirection`, `BBRegime`, `BBSymbolHorizonPoint`, `BBSymbolPoint`, and `BBSymbolSeriesResult`.
- 2026-09-18: Implemented `backend/app/domain/bb/chart_adapter.py` and `backend/app/services/bb_service.py`.
- 2026-09-18: Updated `ports.py`, `sumi_candle_adapter.py`, `fixture_adapter.py`, and `doraemon_adapter.py` to support optional dates and `limit`.
- 2026-09-18: Implemented API schemas in `backend/app/schemas/bb_schema.py` and router in `backend/app/api/bb.py`, registered in `backend/app/main.py`.
- 2026-09-18: Implemented unit and integration test suite `backend/app/tests/test_bb_calculator.py` (13/13 passed).
- 2026-09-18: Executed fast technical gate `verify-v2.ps1` (319 backend tests passed, Alembic migrations passed, frontend lint passed, Vitest 210 tests passed, frontend production build passed).
- 2026-09-18: Executed comprehensive browser UAT `run-comprehensive-uat.ps1` (31/31 scenarios passed, 100%).
- 2026-09-18: Verified database integrity: `backend/sumi.db` SHA-256 hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (zero mutations).
- 2026-09-18: Milestone sealed with independent review `docs/reviews/P5_BB_02_REVIEW.md`.
