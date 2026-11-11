# P5-DATA-01 — Market Data Port and BB Contracts

## Outcome
A robust, provider-neutral market data port and canonical observation layer (`CanonicalBar`) that standardizes daily price, volume, and trading value data for quantitative domains and backtesting, decouples the upcoming Money Flow Blackbox (BB) calculator from vendor-specific payloads and SQLite ORM models, enforces strict data quality invariants (missing vs. zero, price boundary validation, non-silent value fallback), and establishes explicit value-source precedence (`ACTUAL_MATCHED_VALUE` vs `ESTIMATED_TP_X_VOLUME`) under the audited `OHLCV_PROXY` classification.

## Context and problem
- **Addressed requirements:** `FR-CORE-001`, `FR-CORE-012`, `FR-CORE-014`, `NFR-DQ-001`, `DATA-CANDLE-001`, `DATA-CANDLE-002`, `DATA-CANDLE-003`, `TEST-BB-001`, `TEST-BB-002`.
- **Problem:** Quantitative modules currently interact directly with SQLite `Candle` ORM objects or ad-hoc DataFrames. In Money Flow BB calculation, raw price and volume inputs must have verifiable provenance, explicit value-source semantics, and data quality classification.
- **Audit Findings (`DORAEMON_MARKET_DATA_AUDIT.md`):**
  - Daily data is strictly classified as `OHLCV_PROXY`.
  - Actual matched value (`total_trade_value`) is partially populated in upstream sources; missing rows must explicitly use `ESTIMATED_TP_X_VOLUME` rather than silent zero imputation.
  - True Flow (`CLASSIFIED_ORDER_FLOW`, `EXECUTED_ORDER_FLOW`) and Market BB / Breadth remain R&D capabilities until whole-market PIT universe coverage is verified.

## In scope
1. **Canonical Bar & Observation Models** (`backend/app/domain/data/contracts.py`):
   - `CanonicalBar`: standardized bar DTO containing symbol, timestamps, calendar session date, OHLCV, trading value, value source, adjustment type, quality grade, and provider provenance.
   - Core Enums:
     - `ValueSource`: `ACTUAL_MATCHED_VALUE`, `ESTIMATED_TP_X_VOLUME`, `ESTIMATED_CLOSE_X_VOLUME`, `UNAVAILABLE`.
     - `FlowMethod`: `OHLCV_PROXY`, `TICK_TEST_ESTIMATE_RESEARCH_ONLY`, `CLASSIFIED_ORDER_FLOW`, `EXECUTED_ORDER_FLOW`, `UNKNOWN`.
     - `DataQuality`: `HIGH`, `MEDIUM`, `DEGRADED`, `SUSPECT`, `INVALID`.
     - `AdjustmentType`: `UNADJUSTED`, `DIVIDEND_ADJUSTED`, `SPLIT_ADJUSTED`, `FULLY_ADJUSTED`.
   - Invariant validation (`DATA-CANDLE-001/002/003`): `high >= max(open, close)`, `low <= min(open, close)`, `low <= high`, `volume >= 0`.
   - Explicit value calculation policy: `calculate_canonical_trading_value`.
   - Capability Manifest: `MarketDataCapabilityManifest`.
2. **Provider-Neutral Port Interface** (`backend/app/domain/data/ports.py`):
   - `MarketDataPort` protocol defining `get_canonical_bars` and `get_capability_manifest`.
3. **Adapters** (`backend/app/domain/data/adapters/`):
   - `SumiCandleAdapter`: adapts Sumi local `Candle` entities / queries into canonical bars with `ESTIMATED_TP_X_VOLUME` and `OHLCV_PROXY`.
   - `FixtureMarketDataAdapter`: adapts synthetic test matrices and CSV fixtures for deterministic testing.
   - `DoraemonMarketDataAdapter`: adapts Doraemon `/market/prices/{symbol}/history/full` API responses into canonical bars, tagging `ACTUAL_MATCHED_VALUE` where populated and falling back to `ESTIMATED_TP_X_VOLUME`.
4. **BB Domain Contracts Foundation** (`backend/app/domain/bb/contracts.py`):
   - `BBHorizon`: `T03`, `T05`, `T10`, `T20`, `T50`, `T200`.
   - `BBSymbolRequest`: request specification validating accepted methods and time horizons. Rejects incompatible methods.
   - `BBMetricPoint`: canonical BB output point with numerator, denominator, value, quality, value source, and warmup metadata.
   - Invariant guard: keeps Market BB and True Flow explicitly disabled as R&D capabilities.
5. **Comprehensive Unit Test Suite** (`backend/app/tests/test_market_data_contracts.py`):
   - Unit tests covering canonical mapping, value precedence, boundary validation, missing-vs-zero, adapter parity, and method rejection.

## Out of scope
- Implementation of the full rolling BB math calculator (deferred to `P5-BB-02`).
- Market BB / Whole-market Flow Breadth aggregation (deferred to Phase 7).
- Modifying `backend/sumi.db` or writing new persistent database tables.
- Production cron jobs or scraping pipelines in Sumi.

## Invariants
- **No future leaks:** All queries respect strict `as_of` cutoffs.
- **No silent imputation:** Missing actual trading value must never be imputed as 0 or guessed without marking `value_source` as `ESTIMATED_*`.
- **Zero database mutation:** Tests and UAT must use in-memory SQLite (`:memory:`) or isolated temp databases. `backend/sumi.db` must remain untouched.
- **Local-first compliance:** No telemetry or sensitive user data sent externally.
- **Preserve git history:** The 159 historically staged deletions must not be un-staged.

## Architecture and data flow
```mermaid
flowchart TD
    subgraph Data Sources
        SUMI_DB[(Local Sumi DB: candles)]
        DORAEMON_API[Doraemon Read API /market/prices]
        FIXTURE_DATA[CSV / Test Fixtures]
    end

    subgraph Adapters
        SCA[SumiCandleAdapter]
        DMA[DoraemonMarketDataAdapter]
        FDA[FixtureMarketDataAdapter]
    end

    subgraph Market Data Port
        PORT[MarketDataPort Protocol]
        DTO[CanonicalBar & ValueSource Policy]
    end

    subgraph BB & Quantitative Consumers
        BB_CONTRACTS[BB Contracts: BBSymbolRequest, BBMetricPoint]
        BB_CALC[P5-BB-02 Calculator (Upcoming)]
        BACKTEST[Backtest & Signal Engines]
    end

    SUMI_DB --> SCA
    DORAEMON_API --> DMA
    FIXTURE_DATA --> FDA

    SCA --> PORT
    DMA --> PORT
    FDA --> PORT

    PORT --> DTO
    DTO --> BB_CONTRACTS
    DTO --> BB_CALC
    DTO --> BACKTEST
```

## Milestones
1. [x] **Milestone 1**: Implement `backend/app/domain/data/contracts.py` (Enums, `CanonicalBar`, price boundary validator, value source calculator, capability manifest).
2. [x] **Milestone 2**: Implement `backend/app/domain/data/ports.py` (`MarketDataPort` protocol) and adapters in `backend/app/domain/data/adapters/` (`SumiCandleAdapter`, `FixtureMarketDataAdapter`, `DoraemonMarketDataAdapter`).
3. [x] **Milestone 3**: Implement `backend/app/domain/bb/contracts.py` (BB horizons, request, metric point, R&D guardrails).
4. [x] **Milestone 4**: Implement test suite `backend/app/tests/test_market_data_contracts.py` verifying all contracts, adapters, and data quality rules.
5. [x] **Milestone 5**: Execute fast technical gate (`verify-v2.ps1`) and comprehensive browser UAT (`run-comprehensive-uat.ps1`).
6. [x] **Milestone 6**: Independent review seal, ExecPlan completion, and `STATE.json` update.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `FR-CORE-001` / `DATA-CANDLE-001` | `CanonicalBar` schema | `test_market_data_contracts.py::test_canonical_bar_creation_and_attributes` |
| `DATA-CANDLE-002` | `contracts.py::validate_candle_bounds` | `test_market_data_contracts.py::test_price_boundary_validation` |
| `DATA-CANDLE-003` | `contracts.py::calculate_canonical_trading_value` | `test_market_data_contracts.py::test_value_source_precedence` |
| `NFR-DQ-001` | Missing vs zero handling | `test_market_data_contracts.py::test_missing_vs_zero_semantics` |
| `FR-CORE-012` | `MarketDataPort` protocol & adapters | `test_market_data_contracts.py::test_sumi_candle_adapter`, `test_fixture_market_data_adapter`, `test_doraemon_market_data_adapter_parsing` |
| `FR-CORE-014` | Incompatible method rejection | `test_market_data_contracts.py::test_bb_symbol_request_validation_and_incompatible_method_rejection` |
| `TEST-BB-001` | `BBMetricPoint` & `BBSymbolRequest` | `test_market_data_contracts.py::test_bb_metric_point_structure` |
| `TEST-BB-002` | R&D threshold validation guardrail | `test_market_data_contracts.py::test_bb_threshold_semantics_guardrail` |

## Verification commands
```powershell
# 1. Focused market data contracts test suite
.\.venv\Scripts\python.exe -m pytest app/tests/test_market_data_contracts.py -v

# 2. Fast technical gate
.\scripts\verify-v2.ps1

# 3. Comprehensive browser UAT
.\scripts\run-comprehensive-uat.ps1
```

## Rollback and compatibility
The market data port and BB contracts are purely additive domain classes in new packages (`backend/app/domain/data/` and `backend/app/domain/bb/`). No database schema or existing routes are altered. Rollback is a simple removal of the new directories.

## Risks and mitigations
- **Risk:** Upstream API format differences in Doraemon.
  - **Mitigation:** Verified in read-only targeted freshness check (`2026-09-18`). Adapter handles both flat and dictionary-wrapped `{"prices": [...]}` payloads gracefully.
- **Risk:** Value source ambiguity.
  - **Mitigation:** Explicit enum with precedence: `ACTUAL_MATCHED_VALUE` -> `ESTIMATED_TP_X_VOLUME` -> `ESTIMATED_CLOSE_X_VOLUME` -> `UNAVAILABLE`. Never silently impute missing value.

## Progress log
- 2026-09-18: ExecPlan created.
- 2026-09-18: Read `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` and `docs/research/data_capability_matrix.csv`.
- 2026-09-18: Executed read-only targeted freshness check against Doraemon production API for `FPT`, `HPG`, `SSI` (`2026-08-01` -> `2026-09-18`). Verified freshness and resolved stale FPT notice. Updated `data_capability_matrix.csv` and `DORAEMON_MARKET_DATA_AUDIT.md`.
- 2026-09-18: Implemented `backend/app/domain/data/` (`contracts.py`, `ports.py`, and adapters `sumi_candle_adapter.py`, `fixture_adapter.py`, `doraemon_adapter.py`).
- 2026-09-18: Implemented `backend/app/domain/bb/` (`contracts.py` with `BBHorizon`, `BBSymbolRequest`, `BBMetricPoint`, and `validate_bb_threshold_semantics` guardrail).
- 2026-09-18: Implemented unit test suite `backend/app/tests/test_market_data_contracts.py` (10/10 passed).
- 2026-09-18: Verified fast technical gate `verify-v2.ps1` (306 backend tests passed, Alembic migrations passed, frontend lint passed, Vitest 210 tests passed, frontend production build passed).
- 2026-09-18: Verified comprehensive browser UAT `run-comprehensive-uat.ps1` (31/31 scenarios passed, 100%).
- 2026-09-18: Verified database integrity: `backend/sumi.db` SHA-256 hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (zero mutations).
- 2026-09-18: Milestone completed and sealed with independent review `docs/reviews/P5_DATA_01_REVIEW.md`.
