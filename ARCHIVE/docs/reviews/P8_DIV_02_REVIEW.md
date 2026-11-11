# Independent Review Seal: Batch P8-DIV-02 — Confirmed Pivots & Four-Way Multi-Indicator Divergence

**Date**: 2026-09-19  
**Batch ID**: `P8-DIV-02`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P8-DIV-02` delivers strictly causal Confirmed Pivot High/Low detection and Four-Way Divergence detection across three foundational technical oscillators (RSI, MACD Histogram, and Stochastic %K) into the Sumi signal engine. It completely eliminates future lookahead and visual backdating by strictly confirming pivots at $t = p_2 + \text{right\_bars}$ and emitting divergence events with `available_at_index = t` and `availability_event = "BAR_CLOSE"` (`SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 477–490, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7.4, D.10, J.1).

### Key Architectural Accomplishments
1. **Confirmed Pivot Detection (`SIG-DIV-001`)**:
   - Implemented `calculate_confirmed_pivot_high` and `calculate_confirmed_pivot_low` with canonical parameters `left_bars=3`, `right_bars=3` in `backend/app/domain/signals/divergence.py`.
   - A swing high/low at bar $p$ is confirmed strictly when bar $t = p + \text{right\_bars}$ closes.
   - Evaluates strictly causally with zero lookahead.
2. **Four-Way Divergence Across 3 Oscillators (`SIG-DIV-002`)**:
   - Implemented 12 dedicated divergence calculators (4 divergence modes $\times$ 3 oscillators):
     - **Regular Bullish**: Price Lower Low ($P_2 < P_1$) + Oscillator Higher Low ($O_2 > O_1$)
     - **Regular Bearish**: Price Higher High ($P_2 > P_1$) + Oscillator Lower High ($O_2 < O_1$)
     - **Hidden Bullish**: Price Higher Low ($P_2 > P_1$) + Oscillator Lower Low ($O_2 < O_1$)
     - **Hidden Bearish**: Price Lower High ($P_2 < P_1$) + Oscillator Higher High ($O_2 > O_1$)
   - Supported oscillators: RSI (`period=14`), MACD Histogram (`fast=12`, `slow=26`, `signal=9`), Stochastic %K (`k_period=14`, `d_period=3`, `slowing=3`).
3. **Strict Causal Timing & Separation Bounds (`TEST-CAUSAL-002`)**:
   - The divergence event between pivot $p_1$ and $p_2$ is emitted strictly at bar $t = p_2 + \text{right\_bars}$.
   - Enforces separation window bounds $[5, 60]$ bars: $\text{min\_separation} \le p_2 - p_1 \le \text{max\_separation}$.
   - It is never backdated to $p_2$ or $p_1$.
4. **Causal Appending Invariance (`TEST-CAUSAL-001`)**:
   - Appending future bars produces bit-for-bit identical historical divergence signals and confirmation points.
5. **Detailed Factor & Reason Attribution**:
   - Every active signal emits structured reasons detailing pivot indices, confirmation bar, pivot prices, and oscillator readings (e.g. `rsi_reg_bull_div_p1_idx_20_p2_idx_32_p1_24.0_p2_22.0`).
6. **Registry & AST Integration**:
   - All 14 divergence signals registered in `SignalRegistry` with canonical parameter schemas and unique AST aliases (`divergence__*`), bringing total registered signals to 61. Evaluated safely by `SignalBindingAdapter`.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `SIG-DIV-001` (Confirmed Pivots) | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_confirmed_pivot_detection` validates high/low pivot confirmation strictly at $p + 3$. |
| `SIG-DIV-002` (Four-Way Divergence) | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_four_way_divergence_definitions` verifies all 4 types across RSI, MACD Histogram, and Stochastic. |
| `TEST-CAUSAL-002` (Never-Backdated Delay) | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_divergence_never_backdated_causal_delay` confirms zero signal at $p_2$ and activation strictly at $p_2 + 3$. |
| `TEST-CAUSAL-001` (Future Invariance) | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_divergence_future_invariance` validates identical historical outputs when future bars are appended. |
| Separation Bounds ($[5, 60]$ bars) | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_divergence_separation_bounds` verifies suppression below 5 bars and above 60 bars. |
| Equal Pivots Invariant | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_divergence_equal_pivots_handled` validates fail-closed behavior on flat double bottoms/tops. |
| AST DSL Integration | `backend/app/tests/test_divergence_signals.py` | **PASS** | `test_divergence_ast_signal_binding` validates `SignalBindingAdapter.evaluate` on `divergence__*` aliases. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Baseline hash strictly matches: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved untouched. |

---

## 3. Automated Test Evidence

1. **Focused Divergence & Signal Suite**:
   ```text
   pytest app/tests/test_signals.py app/tests/test_signals_api.py app/tests/test_vsa_signals.py app/tests/test_price_signals.py app/tests/test_signal_binding.py app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py app/tests/test_ichimoku_signals.py app/tests/test_divergence_signals.py -v
   ============================= 107 passed in 3.65s =============================
   ```

2. **Fast Technical Gate (`.\scripts\verify-v2.ps1`)**:
   - Backend pytest: 366 passed, 0 failed.
   - Alembic migration integrity: clean on temp database.
   - Frontend ESLint: clean (0 errors, 0 warnings).
   - Frontend Vitest: 32 test files, 210 passed.
   - Frontend Vite build: production bundle built cleanly in 513ms.

3. **Comprehensive Browser E2E UAT (`.\scripts\run-comprehensive-uat.ps1`)**:
   - Domain 1 (Replay & Session Lifecycle): PASS.
   - Domain 2 (Trading Lab & Position Execution): PASS.
   - Domain 3 (Technical Indicators & Multi-Pane Charting): PASS.
   - Domain 4 (Drawing System & Geometry): PASS.
   - Domain 5 (Strategy Tester & 1-Click Battle): PASS.
   - Domain 6 (Navigation, Modals & System Guardrails): PASS.
   - Total results: **31/31 PASSED (100.0%), 0 console errors, 0 DB mutations**.

---

## 4. Audit Conclusion & Approval

Batch `P8-DIV-02` satisfies all functional requirements, causal delay constraints, and non-negotiable architectural invariants. All divergence definitions are fully covered by unit tests, integration tests, and full browser E2E UAT.

**SEAL ISSUED**: `P8-DIV-02` is marked as **ACCEPTED**.  
**RECOMMENDED NEXT TASK**: `P8-COMP-03` (Technical Health, BB events, and strategy namespace integration) per execution roadmap.
