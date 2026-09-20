# Independent Review Seal: Batch P5-DATA-01 — Market Data Port and BB contracts

**Date**: 2026-09-18  
**Batch ID**: `P5-DATA-01`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P5-DATA-01` establishes the provider-neutral market data abstraction layer (`MarketDataPort`, `CanonicalBar`) and foundational Money Flow Blackbox (BB) contracts (`BBHorizon`, `BBSymbolRequest`, `BBMetricPoint`). It enforces strict data quality invariants, formalizes explicit value-source precedence (`ACTUAL_MATCHED_VALUE` vs `ESTIMATED_TP_X_VOLUME`) without silent zero imputation, and codifies R&D boundaries for True Flow and empirical threshold validation.

Key deliverables include:

1. **Canonical Bar & Observation Contracts** (`backend/app/domain/data/contracts.py`):
   - `CanonicalBar`: standardized bar DTO containing symbol, timestamps, calendar session date, OHLCV, trading value, value source, adjustment type, quality grade, and provider provenance (`FR-CORE-001`, `DATA-CANDLE-001`).
   - Domain Enums: `ValueSource` (`ACTUAL_MATCHED_VALUE`, `ESTIMATED_TP_X_VOLUME`, `ESTIMATED_CLOSE_X_VOLUME`, `UNAVAILABLE`), `FlowMethod` (`OHLCV_PROXY`, `TICK_TEST_ESTIMATE_RESEARCH_ONLY`, `CLASSIFIED_ORDER_FLOW`, `EXECUTED_ORDER_FLOW`, `UNKNOWN`), `DataQuality` (`HIGH`, `MEDIUM`, `DEGRADED`, `SUSPECT`, `INVALID`), and `AdjustmentType`.
   - Price Boundary Validation (`DATA-CANDLE-002`): `validate_candle_bounds` strictly asserts $H \ge \max(O, C)$, $L \le \min(O, C)$, $L \le H$, and $V \ge 0$, raising `DataIntegrityError` on violations.
   - Non-Silent Value Source Calculation (`DATA-CANDLE-003`, `NFR-DQ-001`): `calculate_canonical_trading_value` enforces strict fallback precedence and prevents silent zero imputation for missing turnover.
   - Capability Manifest (`MarketDataCapabilityManifest`): records provider capabilities, flow methods, and universe coverage constraints.

2. **Provider-Neutral Port Interface** (`backend/app/domain/data/ports.py`):
   - `MarketDataPort`: clean `typing.Protocol` declaring `get_canonical_bars` and `get_capability_manifest`, decoupling BB and backtest modules from raw database models and vendor API payloads (`FR-CORE-012`).

3. **Provider Adapters** (`backend/app/domain/data/adapters/`):
   - `SumiCandleAdapter`: adapts local SQLite `Candle` entities or DataFrame records into canonical bars with `ESTIMATED_TP_X_VOLUME` and `OHLCV_PROXY`.
   - `FixtureMarketDataAdapter`: adapts CSV files or synthetic in-memory bar matrices for deterministic testing and replay fixtures.
   - `DoraemonMarketDataAdapter`: adapts Doraemon API `/market/prices/{symbol}/history/full` payloads into canonical bars, correctly identifying `ACTUAL_MATCHED_VALUE` when `total_trade_value` is present, and gracefully falling back to typical price estimation when omitted.

4. **BB Domain Contracts Foundation** (`backend/app/domain/bb/contracts.py`):
   - `BBHorizon`: standardized horizons `T03`, `T05`, `T10`, `T20`, `T50`, `T200`.
   - `BBSymbolRequest`: request specification validating accepted methods and horizons (`TEST-BB-001`). Incompatible or unverified methods (`CLASSIFIED_ORDER_FLOW`, `EXECUTED_ORDER_FLOW`) are strictly rejected (`FR-CORE-014`).
   - `BBMetricPoint`: canonical BB point carrying numerator, denominator, value, quality, value source, and warmup metadata.
   - Threshold Guardrail (`validate_bb_threshold_semantics`): explicitly prevents hardcoding 20/30/70/80 thresholds into production without empirical validation (`TEST-BB-002`).

5. **Targeted Freshness Audit** (`docs/research/`):
   - Executed targeted, read-only freshness queries against Doraemon production API on `2026-09-18` for `FPT`, `HPG`, `SSI`.
   - Confirmed data freshness up to `2026-09-18` and updated `docs/research/data_capability_matrix.csv` and `docs/research/DORAEMON_MARKET_DATA_AUDIT.md`.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Command / Target | Result | Evidence |
|---|---|---|---|
| `FR-CORE-001` / `DATA-CANDLE-001` (Canonical Bar Schema) | `test_market_data_contracts.py::test_canonical_bar_creation_and_attributes` | **PASS** | `CanonicalBar` fields and immutability verified. |
| `DATA-CANDLE-002` (Price Bounds & Spread) | `test_market_data_contracts.py::test_price_boundary_validation` | **PASS** | Validates $H \ge \max(O,C)$, $L \le \min(O,C)$, $L \le H$, $V \ge 0$; rejects malformed bars with `DataIntegrityError`. |
| `DATA-CANDLE-003` (Value Source Precedence) | `test_market_data_contracts.py::test_value_source_precedence` | **PASS** | Precedence verified: actual value -> typical price -> close price. |
| `NFR-DQ-001` (Missing vs Zero Handling) | `test_market_data_contracts.py::test_missing_vs_zero_semantics` | **PASS** | Missing value falls back to `ESTIMATED_TP_X_VOLUME`, never silent 0; explicit zero volume yields zero value with `ACTUAL_MATCHED_VALUE`. |
| `FR-CORE-012` (Port & Adapters) | `test_market_data_contracts.py::test_sumi_candle_adapter`, `test_fixture_market_data_adapter`, `test_doraemon_market_data_adapter_parsing` | **PASS** | All three adapters implement `MarketDataPort` protocol and produce valid canonical bar sequences. |
| `FR-CORE-014` (Incompatible Method Rejection) | `test_market_data_contracts.py::test_bb_symbol_request_validation_and_incompatible_method_rejection` | **PASS** | `CLASSIFIED_ORDER_FLOW` and unverified methods raise `ValueError` on request instantiation. |
| `TEST-BB-001` (BB Contracts Structure) | `test_market_data_contracts.py::test_bb_metric_point_structure` | **PASS** | `BBMetricPoint` attributes, value sources, and quality markers validated. |
| `TEST-BB-002` (Threshold Guardrail) | `test_market_data_contracts.py::test_bb_threshold_semantics_guardrail` | **PASS** | 20/30/70/80 thresholds flagged as unvalidated R&D hypotheses. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Hash matches baseline: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Inherited Work Preservation | `git status -s` | **PASS** | Exactly 159 historical staged deletions preserved intact. |

---

## 3. Test & Gate Execution Results

### 3.1 Focused Market Data Contracts Suite
```text
============================= test session starts =============================
app/tests/test_market_data_contracts.py::test_canonical_bar_creation_and_attributes PASSED [ 10%]
app/tests/test_market_data_contracts.py::test_price_boundary_validation PASSED [ 20%]
app/tests/test_market_data_contracts.py::test_value_source_precedence PASSED [ 30%]
app/tests/test_market_data_contracts.py::test_missing_vs_zero_semantics PASSED [ 40%]
app/tests/test_market_data_contracts.py::test_sumi_candle_adapter PASSED [ 50%]
app/tests/test_market_data_contracts.py::test_fixture_market_data_adapter PASSED [ 60%]
app/tests/test_market_data_contracts.py::test_doraemon_market_data_adapter_parsing PASSED [ 70%]
app/tests/test_market_data_contracts.py::test_bb_symbol_request_validation_and_incompatible_method_rejection PASSED [ 80%]
app/tests/test_market_data_contracts.py::test_bb_metric_point_structure PASSED [ 90%]
app/tests/test_market_data_contracts.py::test_bb_threshold_semantics_guardrail PASSED [100%]
======================== 10 passed, 1 warning in 0.10s ========================
```

### 3.2 Fast Technical Gate (`verify-v2.ps1`)
- **Backend Tests**: 306 passed (0 failures).
- **Alembic Upgrade**: Successfully applied migrations on temporary SQLite test DB.
- **Frontend Lint**: Clean (`eslint .`).
- **Frontend Tests**: 32 test files passed, 210 tests passed.
- **Frontend Build**: TypeScript checking and Vite production build passed.

### 3.3 Comprehensive Browser UAT (`run-comprehensive-uat.ps1`)
- **Result**: 31/31 passed (100.0%, 0 failed).
- **Database Integrity Guardrail (`TC-SYS-05`)**: PASSED. SHA-256 verified match.
- **Zero Console Errors (`TC-SYS-04`)**: PASSED.

---

## 4. Architectural Boundaries & Cleanliness

- **Port Decoupling**: Consumers interact only through `MarketDataPort` and `CanonicalBar`, preventing coupling to vendor APIs or database schemas.
- **No ORM Coupling in Domain**: Domain models are pure dataclasses with zero FastAPI or SQLAlchemy dependencies.
- **Strict R&D Boundary**: True Flow and Market BB are explicitly preserved as R&D capabilities, respecting data license and universe limitations.

---

## 5. Audit Review Seal

All acceptance criteria, invariants, and quality gates for `P5-DATA-01` are satisfied in full without regressions or database mutations.

**SEAL STATUS**: **ACCEPTED**  
**RECOMMENDED NEXT TASK**: `P5-BB-02` (Symbol Money Flow Blackbox calculator & rolling indicators under `OHLCV_PROXY`).
