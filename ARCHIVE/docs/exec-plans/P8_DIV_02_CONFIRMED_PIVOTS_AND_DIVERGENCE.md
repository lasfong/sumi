# P8-DIV-02 — Confirmed Pivots and Four-Way Multi-Indicator Divergence

## Outcome
Deliver modular, strictly causal Confirmed Pivot and Four-Way Divergence detection across three core technical oscillators (RSI, MACD Histogram, and Stochastic) into the Sumi signal engine. Ensure zero future lookahead, enforce non-backdated confirmation timing (`TEST-CAUSAL-002`), support separation bounds ($[5, 60]$ bars), provide explicit paired-pivot evidence in signal reasons (`pivot_at`, `confirmed_at`, pivot prices, oscillator levels), and integrate with `SignalRegistry` and `SignalBindingAdapter`.

## Context and problem
- **Canonical References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 477–490, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7.4, D.10 (`SIG-DIV-001`, `SIG-DIV-002`), J.1 (`TEST-CAUSAL-002`, `TEST-CAUSAL-001`); `AGENTS.md`.
- **Problem**:
  1. Traditional technical divergence indicators suffer from massive lookahead and visual backdating: they mark a divergence at the historical pivot bar $p$ when in reality the pivot could not possibly be known until $p + \text{right\_bars}$.
  2. If a backtest executes on a divergence at bar $p$ instead of the confirmation bar $t = p + \text{right\_bars}$, the backtest yields fictional, non-replicable profits.
  3. Divergence calculations must handle separation bounds ($[5, 60]$ bars), equal pivots (flat double bottoms/tops), zero division, and NaN warmup periods fail-closed.
- **Acceptance IDs**:
  - `SIG-DIV-001`: Confirmed Pivot High/Low detection with parameters `left_bars=3`, `right_bars=3`. A pivot at $p$ is confirmed strictly at index $t = p + \text{right\_bars}$.
  - `SIG-DIV-002`: Four-way divergence across RSI, MACD Histogram, and Stochastic:
    - Regular Bullish: Price Lower Low ($P_2 < P_1$) + Oscillator Higher Low ($O_2 > O_1$).
    - Hidden Bullish: Price Higher Low ($P_2 > P_1$) + Oscillator Lower Low ($O_2 < O_1$).
    - Regular Bearish: Price Higher High ($P_2 > P_1$) + Oscillator Lower High ($O_2 < O_1$).
    - Hidden Bearish: Price Lower High ($P_2 < P_1$) + Oscillator Higher High ($O_2 > O_1$).
  - `TEST-CAUSAL-002`: Never-backdated assertion: Signal is emitted at bar $t = p_2 + \text{right\_bars}$ with `available_at_index = t`, `availability_event = "BAR_CLOSE"`. It is never emitted prior to confirmation.
  - `TEST-CAUSAL-001`: Future appending invariance (appending future bars cannot mutate historical divergence signals or confirmation dates).

## In scope
- New domain module `backend/app/domain/signals/divergence.py`:
  - Pure pivot identification: `calculate_confirmed_pivots` (returning list of PivotPoints with `pivot_index`, `confirmed_index`, `pivot_type`, `price`).
  - Pure oscillator calculations: Causal RSI, Causal MACD Histogram, Causal Stochastic.
  - Pure divergence calculators:
    - `calculate_divergence(candles, oscillator_name, divergence_type, ...)`
    - Dedicated calculators for RSI, MACD Histogram, and Stochastic:
      - `calculate_rsi_divergence` (regular/hidden bullish/bearish)
      - `calculate_macd_divergence` (regular/hidden bullish/bearish)
      - `calculate_stoch_divergence` (regular/hidden bullish/bearish)
      - `calculate_confirmed_pivot_high` and `calculate_confirmed_pivot_low`
- Updates to `backend/app/domain/signals/registry.py`:
  - Register all 14 divergence signals with schemas, default parameters (`left_bars=3`, `right_bars=3`, `min_separation=5`, `max_separation=60`), and AST aliases (`divergence__*`).
- Updates to `backend/app/domain/signals/__init__.py`:
  - Export divergence functions.
- Test suite `backend/app/tests/test_divergence_signals.py`:
  - Tests covering pivot confirmation timing, all 4 divergence types across all 3 oscillators, separation bounds, equal pivots, future invariance, and AST rule evaluation.
- Update definition count in `backend/app/tests/test_signals.py`:
  - Update definition count assertion from 47 to 61 (47 + 14 = 61).

## Out of scope
- Multi-signal composite health scoring (`P8-COMP-03`).
- Modifying database schemas or tables in `backend/sumi.db`.

## Invariants
- **Zero Lookahead / Never Backdated**: The divergence point is emitted strictly at `t = p2 + right_bars` with `bar_index = t`, `available_at_index = t`.
- **Reason Attribution**: Reasons must contain `pivot1_idx_{p1}`, `pivot2_idx_{p2}`, `p1_{price1}_p2_{price2}`, and oscillator values.
- **Preserved Database Baseline**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` and 159 staged deletions remain untouched.

## Current architecture
- `backend/app/domain/signals/technical.py`: Provides `compute_causal_rsi`, `compute_causal_macd`.
- `backend/app/domain/signals/registry.py`: Authoritative registry with 47 definitions.
- `backend/app/domain/strategy/signal_binding.py`: Safe AST evaluator.

## Target design
1. **Module `backend/app/domain/signals/divergence.py`**:
   - `ConfirmedPivot`: Data class storing `pivot_index`, `confirmed_index`, `pivot_type` ("HIGH" | "LOW"), `price`, `timestamp`, `confirmed_timestamp`.
   - `find_confirmed_pivots(candles, left_bars=3, right_bars=3)`: Identifies pivots strictly causally. At index $t$, evaluates candidate $p = t - \text{right\_bars}$.
   - Evaluates divergence between consecutive confirmed pivots $(p_1, p_2)$ if $\text{min\_separation} \le p_2 - p_1 \le \text{max\_separation}$.
   - Evaluates Regular Bullish, Hidden Bullish, Regular Bearish, Hidden Bearish.
2. **Registry Integration**:
   - Register 14 divergence signals under category `"divergence"`.

## Milestones
1. **Milestone 1**: Implement `backend/app/domain/signals/divergence.py` with causal pivot identification, 3 oscillators, and 4 divergence types.
2. **Milestone 2**: Register all 14 divergence signals in `SignalRegistry` and update `__init__.py`.
3. **Milestone 3**: Build comprehensive test suite in `backend/app/tests/test_divergence_signals.py` and update registry count in `test_signals.py` (to 61).
4. **Milestone 4**: Execute verification gates (`verify-v2.ps1`, `run-comprehensive-uat.ps1`), verify database hash invariant, emit independent review seal in `docs/reviews/P8_DIV_02_REVIEW.md`, update `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test evidence |
|---|---|---|
| `SIG-DIV-001` | `backend/app/domain/signals/divergence.py` (`calculate_confirmed_pivot_high`, `low`) | `test_divergence_signals.py::test_confirmed_pivot_detection` |
| `SIG-DIV-002` | `backend/app/domain/signals/divergence.py` (4 types x 3 oscillators) | `test_divergence_signals.py::test_four_way_divergence_definitions` |
| `TEST-CAUSAL-002` | Emitted at $t = p_2 + \text{right\_bars}$, never backdated | `test_divergence_signals.py::test_divergence_never_backdated_causal_delay` |
| `TEST-CAUSAL-001` | Pure causal evaluation across all signals | `test_divergence_signals.py::test_divergence_future_invariance` |
| AST DSL Integration | `backend/app/domain/signals/registry.py` (`divergence__*` aliases) | `test_divergence_signals.py::test_divergence_ast_signal_binding` |

## Rollback plan
If any gate or verification fails:
1. Revert changes to `backend/app/domain/signals/registry.py`, `backend/app/domain/signals/__init__.py`, and `backend/app/tests/test_signals.py`.
2. Remove `backend/app/domain/signals/divergence.py` and `backend/app/tests/test_divergence_signals.py`.
3. Re-verify git status and test suite to return to baseline.

## Verification evidence
- **Unit & Integration Tests**:
  - `python -m pytest app/tests/test_divergence_signals.py -v`: 7 passed in 0.08s.
  - Related signal & indicator suites (`test_signals.py`, `test_signals_api.py`, `test_vsa_signals.py`, `test_price_signals.py`, `test_signal_binding.py`, `test_indicators.py`, `test_indicator_parity_e2e.py`, `test_ichimoku_signals.py`, `test_divergence_signals.py`): 107 passed in 3.65s.
- **Fast Technical Gate**:
  - `.\scripts\verify-v2.ps1`:
    - Backend: 366 passed, 0 failed.
    - Alembic migrations: clean on temp database.
    - Frontend ESLint: clean (0 errors, 0 warnings).
    - Frontend Vitest: 32 test files, 210 passed.
    - Vite production build: `dist/` built cleanly in 513ms.
- **Comprehensive Browser E2E UAT**:
  - `.\scripts\run-comprehensive-uat.ps1`: 31/31 passed (100.0%), 0 failed.
  - Console errors: 0.
- **Database Baseline Invariant**:
  - `backend/sumi.db` SHA-256: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (Verified untouched).

