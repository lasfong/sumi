# P8-ICHI-01 — Causal Ichimoku Signal Semantics & Projection Boundary

## Outcome
Deliver strictly causal Ichimoku signal calculations (`ichimoku.score`, `ichimoku.bullish`, `ichimoku.bearish`, `ichimoku.tk_cross_bullish`, `ichimoku.tk_cross_bearish`, `ichimoku.kumo_breakout_bullish`, `ichimoku.kumo_breakout_bearish`) into the Sumi signal engine. Separate visible historical cloud (honoring displacement $D=26$) from projected future cloud, enforce point-in-time causal invariance (`TEST-CAUSAL-001`, `TEST-ICHI-001`), provide explicit factor and reason attributions, and tag replay indicator points with projection metadata so projected future cloud is never mistaken for observed market candles.

## Context and problem
- **Canonical References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 463–475, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7.3, D.9 (`SIG-ICHI-001`), J.1 (`TEST-ICHI-001`, `TEST-CAUSAL-001`); `AGENTS.md`.
- **Problem**:
  1. Standard Ichimoku indicator libraries (including pandas-ta) shift Span A and Span B forward by displacement ($D=26$) into future index timestamps, and shift Chikou backward by $D=26$. If a signal evaluation mistakenly inspects raw spans at bar $t$ without displacement, it compares price against a future cloud that has not yet arrived on the chart.
  2. If replay indicator APIs serialize future-dated projected cloud points without explicit metadata, downstream consumers or replay charts risk interpreting projected lines as observed market bars, violating the local-first replay boundary.
  3. Chikou span is backward-shifted close. In live trading at bar $t$, the trader observes whether current close $C[t]$ is above or below the close $D$ periods ago ($C[t - D]$). A naive implementation that checks Chikou at bar $t + D$ would introduce lookahead bias into historical backtests.
- **Acceptance IDs**:
  - `SIG-ICHI-001`: Ichimoku Score in $[-100.0, 100.0]$ with four core causal factors (Price vs Visible Kumo, Tenkan vs Kijun, Chikou Clearance, Future Kumo Orientation) and optional Kijun Slope; boolean bullish/bearish state; and active reasons attribution.
  - `TEST-ICHI-001`: Historical cloud and displacement fixture proving visible cloud at bar $t$ equals raw spans at $t - D$, with zero lookahead leak.
  - `TEST-CAUSAL-001`: Future appending invariance (evaluating on a prefix of bars yields identical bit-for-bit results after extending history with future bars).

## In scope
- New domain module `backend/app/domain/signals/ichimoku.py`:
  - Pure calculation functions: `calculate_ichimoku_score`, `calculate_ichimoku_bullish`, `calculate_ichimoku_bearish`, `calculate_ichimoku_tk_cross_bullish`, `calculate_ichimoku_tk_cross_bearish`, `calculate_ichimoku_kumo_breakout_bullish`, `calculate_ichimoku_kumo_breakout_bearish`.
- Updates to `backend/app/domain/signals/registry.py`:
  - Register all 7 Ichimoku signals with schemas, default parameters, warmup requirements, causal delay bars, and AST aliases (`ichimoku__*`).
- Updates to `backend/app/domain/signals/__init__.py`:
  - Export the new Ichimoku calculation functions.
- Replay Indicator API update in `backend/app/api/replay.py`:
  - In `get_session_indicators`, add explicit `"is_projected": bool(row_ts > max_visible_ts)` metadata to response records so projected cloud points are distinct from observed candles.
- Test suite `backend/app/tests/test_ichimoku_signals.py`:
  - Comprehensive unit and integration tests verifying formulas, visible cloud displacement, causal Chikou comparison, factor reason attribution, AST rule binding evaluation, and future-invariance proofs.
- Registry count assertion update in `backend/app/tests/test_signals.py`:
  - Update definition count assertion from 40 to 47.

## Out of scope
- Multi-indicator divergence signals (`P8-DIV-02`).
- Composite Technical Health score (`P8-COMP-03`).
- Altering existing LightWeight Charts Ichimoku custom plugin rendering logic in the frontend.
- Modifying database schemas or tables in `backend/sumi.db`.

## Invariants
- **Zero Future Leakage**: Visible cloud at bar $t$ is strictly derived from raw spans at $t - D$ ($D=26$). Chikou clearance at bar $t$ strictly compares $C[t]$ against $C[t - D]$.
- **Authoritative Indicators**: Indicator calculations remain authoritative in backend engines/signals; replay APIs return data strictly aligned with visible replay state.
- **Fail-Closed Warmup**: Prior to index $t < \text{senkou\_period} + \text{displacement} - 1$ ($52 - 1 + 26 = 77$), visible cloud points have quality `INSUFFICIENT_HISTORY` and value `None`.
- **Preserved Database Baseline**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` and 159 staged deletions remain untouched.

## Current architecture
- `backend/app/domain/engine/indicator_engine.py`: Computes Ichimoku using `pandas_ta`, extending the index forward by displacement for chart rendering.
- `backend/app/domain/signals/`: Contains modular signal libraries (`patterns.py`, `regimes.py`, `volume.py`, `structure.py`, `vsa.py`, `technical.py`, `registry.py`).
- `backend/app/api/replay.py`: Computes and serializes indicators dynamically for visible replay session candles.

## Target design
1. **Module `backend/app/domain/signals/ichimoku.py`**:
   - Computes Tenkan ($\frac{\max(H_{9}) + \min(L_{9})}{2}$), Kijun ($\frac{\max(H_{26}) + \min(L_{26})}{2}$), Raw Span A ($\frac{\text{Tenkan} + \text{Kijun}}{2}$), Raw Span B ($\frac{\max(H_{52}) + \min(L_{52})}{2}$).
   - Derives visible spans at bar $t$ by shifting raw spans by displacement $D=26$:
     - $\text{VisibleSpanA}[t] = \text{RawSpanA}[t - D]$
     - $\text{VisibleSpanB}[t] = \text{RawSpanB}[t - D]$
     - $\text{VisibleCloudTop}[t] = \max(\text{VisibleSpanA}[t], \text{VisibleSpanB}[t])$
     - $\text{VisibleCloudBottom}[t] = \min(\text{VisibleSpanA}[t], \text{VisibleSpanB}[t])$
   - Evaluates 4 core factors at bar $t$:
     1. `price_above_cloud` ($C[t] > \text{top}[t]$) vs `price_below_cloud` ($C[t] < \text{bottom}[t]$)
     2. `tk_bullish` ($\text{Tenkan}[t] > \text{Kijun}[t]$) vs `tk_bearish` ($\text{Tenkan}[t] < \text{Kijun}[t]$)
     3. `chikou_above_price` ($C[t] > C[t - D]$) vs `chikou_below_price` ($C[t] < C[t - D]$)
     4. `future_cloud_bullish` ($\text{RawSpanA}[t] > \text{RawSpanB}[t]$) vs `future_cloud_bearish` ($\text{RawSpanA}[t] < \text{RawSpanB}[t]$)
     5. Optional `kijun_slope_positive` ($\text{Kijun}[t] > \text{Kijun}[t-1]$) vs `kijun_slope_negative` ($\text{Kijun}[t] < \text{Kijun}[t-1]$).
   - Normalizes composite score to $[-100.0, +100.0]$: $\text{score} = 100 \times \frac{\text{bullish\_factors} - \text{bearish\_factors}}{N}$.
   - Evaluates boolean signals (`ichimoku.bullish` when score $\ge 50.0$, `ichimoku.bearish` when score $\le -50.0$).
   - Evaluates transition signals (`tk_cross_bullish`, `tk_cross_bearish`, `kumo_breakout_bullish`, `kumo_breakout_bearish`).
2. **Replay Indicator API**:
   - Appends `"is_projected": bool(row_ts > max_visible_ts)` to every serialized record in `get_session_indicators`.

## Milestones
1. **Milestone 1**: Implement `backend/app/domain/signals/ichimoku.py` with pure causal functions and strict warmup handling.
2. **Milestone 2**: Register all 7 Ichimoku signals in `SignalRegistry` and update `__init__.py`.
3. **Milestone 3**: Add projection tagging in `backend/app/api/replay.py`.
4. **Milestone 4**: Build comprehensive test suite in `backend/app/tests/test_ichimoku_signals.py` and update registry count in `test_signals.py`.
5. **Milestone 5**: Execute all verification gates (`verify-v2.ps1`, `run-comprehensive-uat.ps1`), verify database hash invariant, emit independent review seal in `docs/reviews/P8_ICHI_01_REVIEW.md`, update `STATE.json`, and transition to next batch.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test evidence |
|---|---|---|
| `SIG-ICHI-001` | `backend/app/domain/signals/ichimoku.py` (`calculate_ichimoku_score`, `bullish`, `bearish`) | `test_ichimoku_signals.py::test_ichimoku_score_four_factors_and_reasons` |
| `TEST-ICHI-001` | `backend/app/domain/signals/ichimoku.py` (visible cloud = raw spans at $t-D$) | `test_ichimoku_signals.py::test_ichimoku_visible_cloud_displacement_fixture` |
| `TEST-CAUSAL-001` | Pure causal evaluation across all signals | `test_ichimoku_signals.py::test_ichimoku_future_invariance` |
| Replay Projection Boundary | `backend/app/api/replay.py` (`is_projected` tagging) | `test_ichimoku_signals.py::test_replay_indicator_projection_tagging` |
| AST DSL Integration | `backend/app/domain/signals/registry.py` (`ichimoku__*` aliases) | `test_ichimoku_signals.py::test_ichimoku_ast_signal_binding` |

## Rollback plan
If any gate or verification fails:
1. Revert changes to `backend/app/domain/signals/ichimoku.py`, `backend/app/domain/signals/registry.py`, `backend/app/domain/signals/__init__.py`, `backend/app/api/replay.py`, and `backend/app/tests/test_signals.py`.
2. Remove `backend/app/tests/test_ichimoku_signals.py`.
3. Re-verify git status and test suite to return to baseline.

## Verification evidence
- **Unit & Integration Tests**:
  - `python -m pytest app/tests/test_ichimoku_signals.py -v`: 10 passed in 0.54s.
  - Related signal & indicator suites (`test_signals.py`, `test_signals_api.py`, `test_vsa_signals.py`, `test_price_signals.py`, `test_signal_binding.py`, `test_indicators.py`, `test_indicator_parity_e2e.py`, `test_ichimoku_signals.py`): 100 passed in 4.17s.
- **Fast Technical Gate**:
  - `.\scripts\verify-v2.ps1`:
    - Backend: 359 passed, 0 failed.
    - Alembic migrations: clean on temp database.
    - Frontend ESLint: clean (0 errors, 0 warnings).
    - Frontend Vitest: 32 test files, 210 passed.
    - Vite production build: `dist/` built cleanly in 551ms.
- **Comprehensive Browser E2E UAT**:
  - `.\scripts\run-comprehensive-uat.ps1`: 31/31 passed (100.0%), 0 failed.
  - Console errors: 0.
- **Database Baseline Invariant**:
  - `backend/sumi.db` SHA-256: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (Verified untouched).

