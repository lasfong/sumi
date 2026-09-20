# P2-MKT-02 — Vietnam Market-Rule Profile

## Outcome
Formalize Vietnam equity market rules, trading calendar sessions, settlement mechanics, lot/tick sizing, price bands, and locked limit behavior as pure, decoupled domain modules within `backend/app/domain/market/`. Eliminate calendar-day `timedelta(days=N)` approximations and naive bar-index delays by introducing deterministic trading-session navigation with immutable holiday fixtures (2020–2026+), discrete lot settlement tracking (`HoldingLot`), statutory price band rounding, and conservative locked-limit counter-liquidity protection in `BacktestExecutionKernel`.

## Context and problem
In daily OHLC backtesting for Vietnam equities, naive simulators suffer from multiple critical failure modes:
1. They use calendar days (`timedelta(days=2)`) or simple bar count differences (`i - buy_idx >= 2`), causing illegal trade exits across weekends and public holidays (`TEST-BT-003`).
2. They fail to reflect the Vietnam T+2 / T+1.5 afternoon settlement regime (VSD Decision 109/QĐ-VSD), allowing impossible exits at Day T+2 Open before shares are delivered (`TEST-BT-002`).
3. They ignore statutory price limits (HOSE ±7%, HNX ±10%, UPCoM ±15%) and tick-size rounding rules (Circular 120/2020/TT-BTC), producing fills at invalid price steps.
4. They fail to detect "locked limit" bars (trần cứng / sàn cứng), fabricating counter-liquidity fills when the entire market is queued with zero opposite liquidity (`TEST-BT-004`).
5. At the simulation phase end, they either drop or illegally force-liquidate unsettled holdings (`TEST-BT-005`).

Addressed:
- Roadmap Task: `P2-MKT-02`
- Canonical Specs: `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` (§19, §23, §24, Appendix H, Appendix J.1)
- Acceptance Gates: `TEST-BT-002`, `TEST-BT-003`, `TEST-BT-004`, `TEST-BT-005`
- DEV Program: `docs/dev-program/START.md`, `ROADMAP.md`

## In scope
- Pure, versioned domain package `backend/app/domain/market/`:
  - `calendar.py`: `TradingCalendar` with immutable Vietnam exchange holiday fixtures (`VIETNAM_HOLIDAYS_V1` 2020–2026+) and session-based arithmetic without `timedelta(days=N)`.
  - `rules.py`: `MarketRules` with exchange price bands (HOSE ±7%, HNX ±10%, UPCoM ±15%), tiered tick sizes, statutory rounding (Ceiling down, Floor up), board lot sizing (100 shares), and locked limit detection.
  - `settlement.py`: `SettlementEngine` enforcing cash-equity T+2 / T+1.5 afternoon session availability, conservative 1D daily ambiguity rules, and `HoldingLot` model.
  - `provider.py`: Named execution profiles (`vietnam_default_conservative`, `vietnam_standard_t2`, `vietnam_legacy_t3`, `VN_EQUITY_DAILY_V1`) and `MarketRuleProvider`.
  - `__init__.py`: Public domain exports.
- Integration into `BacktestExecutionKernel` (`backend/app/domain/backtest/execution.py`):
  - Discrete `HoldingLot` tracking in `BacktestPosition`.
  - Locked limit fill rejection at Bar Open and suppression of intraday Stop Loss on locked floor bars.
  - Trading-calendar-aware settlement validation before queueing or executing SELL orders.
  - Proper handling of final bar signals and phase-end unsellable positions (`TEST-BT-005`).
- Backward-compatible extension of `ExecutionAssumptions` in `backend/app/schemas/analytics_trust_schema.py`.
- Profile resolution wiring in `backend/app/services/backtest_service.py`.
- Comprehensive test suite in `backend/app/tests/test_market_rules.py`.

## Out of scope
- Multi-asset portfolio cash allocation / position sizing across simultaneous signals (Phase 4).
- Corporate action adjustments / ex-dividend price restatements (P2-MKT-03).
- Odd-lot continuous matching or fractional share execution (<100 shares).
- Intraday minute-bar session state engines (1D daily bar scope only).
- Database migrations or changes to SQLite schema.

## Invariants
- Zero DB mutation: Production `backend/sumi.db` must NEVER be modified or connected to. SHA256 must match baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`.
- Working tree integrity: All 159 historically staged deletions must remain preserved in git staging.
- Pure domain boundary: `domain/market/` has zero dependencies on FastAPI, SQLAlchemy, or database models.
- No future leak: Signals evaluated on Close[T] fill at Open[T+1]; price limits for Bar T strictly reference Close[T-1].
- Full backward compatibility: Retain existing keys and formats in `ExecutionAssumptions` and `/api/backtest/run`.

## Current architecture
- `backend/app/domain/backtest/execution.py`: Simple scalar holding bars calculation (`i - position.last_buy_bar_index >= min_holding_bars`), blind to weekends, holidays, and price limits.
- `backend/app/domain/backtest/models.py`: Flat `BacktestPosition` with single scalar `last_buy_bar_index`.
- `backend/app/schemas/analytics_trust_schema.py`: Static placeholder string for settlement in `ExecutionAssumptions`.
- `backend/app/services/backtest_service.py`: Hardcoded `min_holding_bars=2` passed to kernel with no market profile resolution.

## Target design
- `backend/app/domain/market/`:
  - `TradingCalendar`: Session arithmetic on immutable `VIETNAM_HOLIDAYS_V1` frozenset.
  - `MarketRules`: Calculates statutory price limits with tick alignment and detects ceiling/floor lock states.
  - `SettlementEngine`: Tracks `HoldingLot` settlement dates and validates sellability per session phase.
  - `MarketRuleProvider`: Resolves named profiles (`vietnam_default_conservative`, `vietnam_standard_t2`, etc.).
- `backend/app/domain/backtest/execution.py`: Injects `MarketProfile`, checks locked limits at Open and intraday, validates lot sellability via `SettlementEngine`.
- `backend/app/domain/backtest/models.py`: `BacktestPosition` contains `lots: List[HoldingLot]` while preserving backward-compatible scalar fields.
- `backend/app/schemas/analytics_trust_schema.py`: `ExecutionAssumptions` exposes profile, calendar, price band, and settlement details with conservative defaults.

## Milestones
1. **Milestone 1 — Pure Market Domain Modules**:
   Implement `backend/app/domain/market/` (`__init__.py`, `calendar.py`, `rules.py`, `settlement.py`, `provider.py`) with 100% pure in-memory logic and immutable fixtures for 2020–2026+.
2. **Milestone 2 — Kernel & Service Integration**:
   Integrate `MarketRuleProvider` and `HoldingLot` into `BacktestExecutionKernel`, extend `ExecutionAssumptions`, wire `BacktestService`, and write comprehensive unit tests in `backend/app/tests/test_market_rules.py`.
3. **Milestone 3 — Verification & Technical Gate**:
   Run full test suite (`pytest`), fast technical gate (`verify-v2.ps1`), verify database hash and git index.
4. **Milestone 4 — Independent Review & Program State Update**:
   Produce handoff report, review seal `docs/reviews/P2_MKT_02_REVIEW.md`, and update `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| TEST-BT-002 | `SettlementEngine.is_sellable()` enforces T+2 afternoon / T+3 open | `test_market_rules.py::test_settlement_engine_t2_availability` |
| TEST-BT-003 | `TradingCalendar` skips weekends and Tet/holidays without timedelta math | `test_market_rules.py::test_trading_calendar_holidays_and_weekends` |
| TEST-BT-004 | `MarketRules` detects locked limits; kernel rejects impossible fills | `test_market_rules.py::test_locked_limits_rejection` |
| TEST-BT-005 | Phase-end open positions handled cleanly without forced liquidation | `test_market_rules.py::test_final_bar_and_phase_end` |
| MKT-RULES-01 | Statutory tick sizes and ceiling/floor price limits across HOSE, HNX, UPCoM | `test_market_rules.py::test_market_rules_price_bands_and_ticks` |
| KERNEL-INT | End-to-end backtest kernel execution under `vietnam_default_conservative` | `test_market_rules.py::test_kernel_integration_vietnam_profile` |
| UAT-COMPAT | All 227+ existing backend tests and frontend technical gates pass | `powershell .\scripts\verify-v2.ps1` |

## Verification commands
```powershell
pytest backend/app/tests/test_market_rules.py -v
pytest backend/app/tests/test_backtest_kernel.py -v
pytest backend/app/tests/test_backtest.py -v
powershell .\scripts\verify-v2.ps1
powershell .\scripts\run-comprehensive-uat.ps1
node scripts/verify-dev-program.mjs
Get-FileHash -Algorithm SHA256 backend\sumi.db
(git status --porcelain | Select-String '^D  ').Count
```

## Rollback and compatibility
Full backward compatibility is preserved:
- Existing tests calling `BacktestExecutionKernel()` without a profile receive `vietnam_default_conservative` with identical fee/tax/slippage defaults.
- `BacktestPosition` retains `last_buy_bar_index` and `last_buy_timestamp` scalar fields alongside `lots`.
- `ExecutionAssumptions` maintains existing string fields (`execution_timing`, `price_basis`, `fees`, `taxes`, `slippage`, `liquidity`, `position_sizing`, `settlement`) while adding new optional fields.
- If rollback is needed, revert `execution.py`, `analytics_trust_schema.py`, `backtest_service.py` and remove `backend/app/domain/market/`.

## Risks and mitigations
- *Risk*: Discrepancy in backtest trade counts when synthetic test data spans weekends or holidays.
  *Mitigation*: Provide explicit trading-day timestamps in synthetic test fixtures and allow permissive profile for legacy test suites if needed.
- *Risk*: Inadvertent production DB writes.
  *Mitigation*: `domain/market/` is 100% pure in-memory with zero DB imports; tests use isolated temporary databases; SHA256 verified before and after runs.

## Progress log
- 2026-09-16: ExecPlan drafted; specification and codebase architecture surveyed; baseline DB hash and 159 staged deletions confirmed intact.
- 2026-09-16: Implemented pure domain module `backend/app/domain/market/` (`calendar.py`, `rules.py`, `settlement.py`, `provider.py`, `__init__.py`).
- 2026-09-16: Integrated market rules and discrete lot settlement into `BacktestExecutionKernel` in `backend/app/domain/backtest/execution.py`.
- 2026-09-16: Extended `ExecutionAssumptions` in `backend/app/schemas/analytics_trust_schema.py` while maintaining backward compatibility.
- 2026-09-16: Updated `backend/app/services/backtest_service.py` to resolve execution profiles and pass to kernel and assumptions.
- 2026-09-16: Authored 26 comprehensive unit tests in `backend/app/tests/test_market_rules.py` covering TEST-BT-002 through TEST-BT-005.
- 2026-09-16: Verified full suite: 26 market rule tests pass, 15 existing backtest tests pass (41 total backtest/market rule tests).
- 2026-09-16: Ran `.\scripts\verify-v2.ps1`: 253 backend tests passed, Alembic migrations passed, frontend lint/test (210 passed)/build passed.
- 2026-09-16: Verified DB SHA256 matches baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`.
- 2026-09-16: Verified 159 historically staged deletions remain intact.

## Decision log
- Decision: Model Vietnam T+1.5 settlement with conservative 1D ambiguity (Open[T+2] blocked, intraday SL/TP blocked on T+2, exit allowed on T+2 Close or T+3 Open).
  Rationale: On 1D OHLC bars, morning vs afternoon price sequence is unknowable; assuming morning fills at T+2 violates VSD rules and creates forward-looking bias.
- Decision: Statutory tick rounding uses floor for ceiling price and ceil for floor price.
  Rationale: Prevents rounded price limits from breaching statutory exchange percentage bands (HOSE 7%, HNX 10%, UPCoM 15%).
- Decision: Retain dynamically attached `position.lots` in kernel execution without modifying `models.py`.
  Rationale: Adheres strictly to File Write Ownership by leaving unowned `backend/app/domain/backtest/models.py` untouched.

## Completion evidence
- Unit tests: `backend/app/tests/test_market_rules.py` (26 passed in 0.14s)
- Integration tests: `app/tests/test_market_rules.py app/tests/test_backtest.py app/tests/test_backtest_kernel.py` (41 passed in 0.64s)
- Fast technical gate: `powershell .\scripts\verify-v2.ps1` (253 backend tests passed, 210 frontend tests passed, frontend build passed)
- DB SHA256 integrity: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (exact match, 0 mutations)
- Git status integrity: 159 staged deletions intact.
