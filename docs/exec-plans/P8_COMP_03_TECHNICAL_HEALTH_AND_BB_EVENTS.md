# P8-COMP-03 — Technical Health, BB Events, and Strategy Namespace Integration

## Outcome
Deliver composable multi-signal Technical Health scoring (`SIG-HEALTH-001`), promoted Money Flow BB events and signals (`BBI-SIG-001`, `BBI-SIG-002`, `BBI-SIG-003`), and safe strategy binding namespace integration with BB method and minimum-quality constraints (`FR-CORE-004`, `FR-CORE-005`, `FR-CORE-006`, `FR-CORE-007`, `TEST-DSL-001`). Ensure zero lookahead, enforce non-overlapping family weighting, handle missing BB neutral/absent semantics without negative penalty, prevent hardcoded research thresholds from becoming frozen production rules, enforce method/quality guards, reject arbitrary AST escapes, and maintain 100% backward compatibility with existing strategies.

## Context and problem
- **Canonical References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 491–504, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7, D.8 (`SIG-HEALTH-001`), D.11 (`BBI-SIG-001/002/003`), J.1 (`TEST-DSL-001`), `BB_BEHAVIOR_MEASUREMENT_P6_01.md`, and `AGENTS.md`.
- **Problem**:
  1. Technical analysis systems frequently average arbitrary indicators without regard for collinearity (e.g. averaging RSI, CCI, Stoch, and MACD simultaneously, heavily double-counting momentum). A robust Technical Health score must summarize **four distinct, non-overlapping information families**: Trend, Momentum, Transition, and Participation.
  2. In markets or regimes where Money Flow Blackbox (BB) is unavailable or absent, an indicator must not silently penalize the score as negative evidence. Missing BB must have explicit neutral/absent semantics.
  3. Technical practitioners often borrow 20/30/70/80 thresholds for BB without statistical foundation. `P6-MEASURE-01` verified these must NOT be hardcoded into production trading rules. Instead, BB events must be parameter-driven or regime-based (e.g. rising/falling, positive/negative regime relative to 50.0, confluence).
  4. When strategies consume BB signals, they must enforce method authorization (only `OHLCV_PROXY` is validated for daily symbol data; `TRUE_FLOW` and `TICK_TEST` remain unvalidated R&D) and minimum data quality to prevent silent semantic corruption across tickers and periods.
  5. The DSL and Strategy Rule Evaluator must combine signals strictly via safe AST without widening the grammar or exposing execution vulnerabilities (`TEST-DSL-001`).

## In scope
1. **Technical Health Scoring (`backend/app/domain/signals/health.py`)**:
   - `calculate_technical_health_score`: Pure function returning score in $[-100.0, +100.0]$ based on four non-overlapping families:
     - `trend`: Price relative to EMA20 / EMA50 and EMA slope.
     - `momentum`: RSI (period 14).
     - `transition`: MACD Histogram / Signal state (positive/negative, expanding/contracting).
     - `participation`: Relative Volume OR BB flow confirmation if available.
   - Versioned weight preset (default `health_v1_balanced`: `{"trend": 0.35, "momentum": 0.25, "transition": 0.20, "participation": 0.20}`).
   - Missing BB / Volume semantics: BB absence is not negative evidence; if BB is absent, participation utilizes volume or neutral re-weighting with explicit `bb_absent_neutral` reason.
   - States: `health.score` (float), `health.favorable` (`score >= +35.0`), `health.unfavorable` (`score <= -35.0`).
2. **Promoted Money Flow BB Events (`backend/app/domain/signals/flow_events.py`)**:
   - Consumes authoritative `ProxyBBCalculator` and BB contracts read-only.
   - Direction signals: `bb.direction_rising` and `bb.direction_falling` across horizons (`T03`, `T05`, `T10`, `T20`, `T50`, `T200`).
   - Regime signals: `bb.regime_positive` and `bb.regime_negative` (relative to neutral 50.0 line).
   - Multi-horizon confluence: `bb.confluence_bullish` and `bb.confluence_bearish` (e.g. T05 and T20 alignment).
   - Parameterized turn events: `bb.turn_up` and `bb.turn_down` with required explicit parameters (`threshold`, `tolerance`), enforcing `validate_bb_threshold_semantics`.
   - Method & Quality guardrails (`BBI-SIG-003`): validates permitted `flow_method` (defaults to `FlowMethod.OHLCV_PROXY`) and minimum `data_quality` (`DataQuality.HIGH` / `MEDIUM`), failing closed with `INVALID_FLOW_METHOD` or `INSUFFICIENT_HISTORY` if violated.
3. **Signal Registry Integration (`backend/app/domain/signals/registry.py`)**:
   - Register 3 health signals (`health.score`, `health.favorable`, `health.unfavorable`) with AST aliases `health__*`.
   - Register 8 BB flow signals (`bb.direction_rising`, `bb.direction_falling`, `bb.regime_positive`, `bb.regime_negative`, `bb.confluence_bullish`, `bb.confluence_bearish`, `bb.turn_up`, `bb.turn_down`) with AST aliases `bb__*`.
   - Update `SignalRegistry` definitions and export functions in `__init__.py`.
4. **Strategy Namespace Integration & Guardrails (`backend/app/domain/strategy/`)**:
   - Update `strategy_schema.py`: add optional `flow_constraints` (`FlowConstraintsConfig` specifying `accepted_methods` and `min_quality`).
   - Update `signal_binding.py`: enforce flow constraints when BB signals are evaluated; ensure strict AST safety (`TEST-DSL-001`).
5. **Comprehensive Test Suite**:
   - `backend/app/tests/test_health_and_flow_signals.py`: Tests covering Technical Health 4-family scoring, no double counting, missing BB neutral semantics, BB flow events, method/quality guardrails, AST binding, and adversarial AST rejection.
   - Update registry count in `backend/app/tests/test_signals.py`.

## Out of scope
- Hardcoding unvalidated 20/30/70/80 threshold constants as production defaults.
- Database schema changes in `backend/sumi.db`.
- Modifying the safe AST grammar in `rule_evaluator.py`.

## Invariants
- **Non-Overlapping Families**: Momentum does not combine both RSI and CCI with high weights; trend and momentum remain separate.
- **BB Absence Is Neutral**: Absence of BB data never lowers health score; audit reasons record `bb_absent_neutral`.
- **Method & Quality Isolation**: Only authorized daily methods (`OHLCV_PROXY`) are evaluated; unvalidated methods fail closed.
- **AST Whitelist Safety (`TEST-DSL-001`)**: Arbitrary calls, imports, and attribute lookups remain strictly rejected.
- **Preserved Database Baseline**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` and 159 staged deletions remain untouched.

## Current architecture
- `backend/app/domain/signals/`: Pure signal modules (`volume.py`, `patterns.py`, `regimes.py`, `support_resistance.py`, `structure.py`, `technical.py`, `vsa.py`, `ichimoku.py`, `divergence.py`).
- `backend/app/domain/bb/`: `calculator.py`, `contracts.py`, `market_aggregator.py`.
- `backend/app/domain/strategy/signal_binding.py`: AST alias validation, snapshot integrity check, fail-closed evaluation.

## Target design
1. **Module `backend/app/domain/signals/health.py`**:
   - Computes 4 distinct non-overlapping family subscores:
     - $S_{\text{trend}} \in [-1, 1]$
     - $S_{\text{momentum}} \in [-1, 1]$
     - $S_{\text{transition}} \in [-1, 1]$
     - $S_{\text{participation}} \in [-1, 1]$
   - Computes weighted composite score $\in [-100, 100]$.
   - Favorable / unfavorable state booleans.
   - Causal warmup of 50 bars.
2. **Module `backend/app/domain/signals/flow_events.py`**:
   - Converts `CandleBar` sequence to `CanonicalBar` with typical price trading value.
   - Computes BB series via `ProxyBBCalculator.calculate_series`.
   - Evaluates direction, regime, confluence, and turn events.
   - Validates method and quality constraints.
3. **Registry & Strategy Schema**:
   - Register all 11 new signals in `SignalRegistry`.
   - Support `flow_constraints` in `StrategyConfig`.
   - Safely bind in `SignalBindingAdapter`.

## Milestones
1. **Milestone 1**: Implement `backend/app/domain/signals/health.py` with 4-family scoring, versioned presets, and missing-BB neutral semantics.
2. **Milestone 2**: Implement `backend/app/domain/signals/flow_events.py` with pure BB event calculation, method/quality guards, and parameterized turn events.
3. **Milestone 3**: Register 11 new signals in `SignalRegistry`, export in `__init__.py`, update `strategy_schema.py` and `signal_binding.py`.
4. **Milestone 4**: Implement test suite `backend/app/tests/test_health_and_flow_signals.py` and update registry count in `test_signals.py` (to 72).
5. **Milestone 5**: Execute verification gates (`verify-v2.ps1`, `run-comprehensive-uat.ps1`), verify database hash invariant, emit independent review seal in `docs/reviews/P8_COMP_03_REVIEW.md`, update `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test evidence |
|---|---|---|
| `SIG-HEALTH-001` | `backend/app/domain/signals/health.py` | `test_health_and_flow_signals.py::test_technical_health_four_families_and_reasons`, `test_health_missing_bb_neutral_semantics` |
| `BBI-SIG-001` | `backend/app/domain/signals/flow_events.py` | `test_health_and_flow_signals.py::test_bb_direction_and_regime_events` |
| `BBI-SIG-002` | Parameterized turn events, no hardcoded defaults | `test_health_and_flow_signals.py::test_bb_turn_events_require_parameters` |
| `BBI-SIG-003` | Method and quality constraint validation | `test_health_and_flow_signals.py::test_bb_method_and_quality_guards` |
| `FR-CORE-004/005` | Precomputed booleans, safe AST binding | `test_health_and_flow_signals.py::test_health_and_bb_ast_strategy_binding` |
| `TEST-DSL-001` | AST whitelist safety and adversarial escape rejection | `test_health_and_flow_signals.py::test_dsl_safety_adversarial_rejection` |

## Rollback plan
If any gate or verification fails:
1. Revert changes to `backend/app/domain/signals/registry.py`, `backend/app/domain/signals/__init__.py`, `strategy_schema.py`, `signal_binding.py`, and `test_signals.py`.
2. Remove `backend/app/domain/signals/health.py`, `flow_events.py`, and `backend/app/tests/test_health_and_flow_signals.py`.
3. Re-verify git status and test suite to return to baseline.

## Verification evidence
- **Focused Test Suite**:
  - `pytest backend/app/tests/test_health_and_flow_signals.py backend/app/tests/test_signals.py backend/app/tests/test_signals_api.py backend/app/tests/test_vsa_signals.py backend/app/tests/test_price_signals.py backend/app/tests/test_signal_binding.py backend/app/tests/test_indicators.py backend/app/tests/test_indicator_parity_e2e.py backend/app/tests/test_ichimoku_signals.py backend/app/tests/test_divergence_signals.py -v`: 118 passed in 4.15s.
- **Fast Technical Gate (`verify-v2.ps1`)**:
  - Backend pytest: 422 passed, 0 failed.
  - Alembic migrations: clean on temp DB.
  - Frontend ESLint: clean (0 errors, 0 warnings).
  - Frontend Vitest: 32 test files, 210 passed.
  - Frontend Vite build: production bundle built cleanly in 682ms.
- **Comprehensive Browser E2E UAT (`run-comprehensive-uat.ps1`)**:
  - 31/31 passed (100.0%), 0 console errors, 0 DB mutations.
- **Database Baseline & Staged Deletions**:
  - `backend/sumi.db` SHA-256: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (Verified match).
  - Exactly 159 staged deletions intact.
- **Independent Review Seal**:
  - `docs/reviews/P8_COMP_03_REVIEW.md` (Status: ACCEPTED).

