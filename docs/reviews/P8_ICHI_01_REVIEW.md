# Independent Review Seal: Batch P8-ICHI-01 — Causal Ichimoku Signal Semantics & Projection Boundary

**Date**: 2026-09-19  
**Batch ID**: `P8-ICHI-01`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P8-ICHI-01` delivers strictly causal Ichimoku signal calculations (`ichimoku.score`, `ichimoku.bullish`, `ichimoku.bearish`, `ichimoku.tk_cross_bullish`, `ichimoku.tk_cross_bearish`, `ichimoku.kumo_breakout_bullish`, `ichimoku.kumo_breakout_bearish`) into the Sumi signal engine. It separates the visible historical cloud (honoring displacement $D=26$) from projected future cloud, enforces point-in-time causal invariance (`TEST-CAUSAL-001`, `TEST-ICHI-001`), provides explicit factor and reason attributions, and tags replay indicator points with projection metadata so projected future cloud is never mistaken for observed market candles (`SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 463–475, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7.3, D.9, J.1).

### Key Architectural Accomplishments
1. **Strictly Causal Multi-Factor Score (`SIG-ICHI-001`)**:
   - Implemented pure function `calculate_ichimoku_score` in `backend/app/domain/signals/ichimoku.py`.
   - Four core causal factors evaluated strictly at bar $t$:
     1. Price relative to visible Kumo: $C[t] > \text{VisibleCloudTop}[t]$ (bullish) vs $C[t] < \text{VisibleCloudBottom}[t]$ (bearish).
     2. Tenkan vs Kijun: $\text{Tenkan}[t] > \text{Kijun}[t]$ (bullish) vs $\text{Tenkan}[t] < \text{Kijun}[t]$ (bearish).
     3. Chikou clearance: $C[t] > C[t - D]$ (bullish) vs $C[t] < C[t - D]$ (bearish) using causal comparison against close $D=26$ bars ago.
     4. Future Kumo orientation known at $t$: $\text{RawSpanA}[t] > \text{RawSpanB}[t]$ (bullish) vs $\text{RawSpanA}[t] < \text{RawSpanB}[t]$ (bearish).
     5. Optional Kijun slope: $\text{Kijun}[t] > \text{Kijun}[t-1]$ (when `include_kijun_slope=True`).
   - Normalizes composite score to $[-100.0, +100.0]$: $\text{score} = 100 \times \frac{\text{bullish\_factors} - \text{bearish\_factors}}{N}$.
2. **Visible Cloud Displacement Fixture (`TEST-ICHI-001`)**:
   - For displacement $D=26$, the visible cloud top/bottom active at bar $t$ strictly equals $\max(\text{RawSpanA}[t - D], \text{RawSpanB}[t - D])$ and $\min(\dots)$.
   - Warmup requirement: $t < 52 - 1 + 26 = 77$ bars yields quality `INSUFFICIENT_HISTORY` and value `None`.
3. **Transition Signals**:
   - `ichimoku.tk_cross_bullish` / `bearish`: Tenkan strictly crosses above/below Kijun.
   - `ichimoku.kumo_breakout_bullish` / `bearish`: Close strictly crosses above visible cloud top / below visible cloud bottom.
4. **Causal Appending Invariance (`TEST-CAUSAL-001`)**:
   - Appending future bars produces bit-for-bit identical historical outputs across all 7 Ichimoku signals.
5. **Replay Indicator API Projection Tagging**:
   - Updated `backend/app/api/replay.py` (`get_session_indicators`) to tag any points with `row_ts > max_visible_ts` with explicit `"is_projected": True` metadata, preserving clean backward compatibility for observed replay candle rows.
6. **Registry & AST Integration**:
   - All 7 signals registered in `SignalRegistry` with canonical parameter schemas and unique AST aliases (`ichimoku__*`), bringing total registered signals to 47. Evaluated safely by `SignalBindingAdapter`.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `SIG-ICHI-001` (Ichimoku Score & Bullish/Bearish) | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_ichimoku_score_four_factors_and_reasons`, `test_ichimoku_bullish_and_bearish_states` validate $[-100, 100]$ score and active reasons. |
| `TEST-ICHI-001` (Displacement Fixture) | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_ichimoku_visible_cloud_displacement_fixture` confirms visible cloud at bar $t$ equals raw spans at $t - 26$. |
| `TEST-CAUSAL-001` (Future Invariance) | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_ichimoku_future_invariance` validates bit-for-bit identical outputs when 50 future bars are appended. |
| Crossover & Breakout Signals | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_ichimoku_tk_cross`, `test_ichimoku_kumo_breakout` validate causal event triggers. |
| AST DSL Integration | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_ichimoku_ast_signal_binding` validates `SignalBindingAdapter.evaluate` on `ichimoku__*` aliases. |
| Replay Indicator Projection Tagging | `backend/app/tests/test_ichimoku_signals.py` | **PASS** | `test_replay_indicator_projection_tagging` validates 80 visible points and 26 projected points with `"is_projected": True`. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Baseline hash strictly matches: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved untouched. |

---

## 3. Automated Test Evidence

1. **Focused Signal & Indicator Suite**:
   ```text
   pytest app/tests/test_signals.py app/tests/test_signals_api.py app/tests/test_vsa_signals.py app/tests/test_price_signals.py app/tests/test_signal_binding.py app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py app/tests/test_ichimoku_signals.py -v
   ============================= 100 passed in 4.17s =============================
   ```

2. **Fast Technical Gate (`.\scripts\verify-v2.ps1`)**:
   - Backend pytest: 359 passed, 0 failed.
   - Alembic migration integrity: clean on temp database.
   - Frontend ESLint: clean (0 errors, 0 warnings).
   - Frontend Vitest: 32 test files, 210 passed.
   - Frontend Vite build: production bundle built cleanly in 551ms.

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

Batch `P8-ICHI-01` satisfies all functional and non-negotiable architectural requirements. All acceptance criteria (`SIG-ICHI-001`, `TEST-ICHI-001`, `TEST-CAUSAL-001`, replay projection boundary) are fully verified by automated tests and browser E2E UAT.

**SEAL ISSUED**: `P8-ICHI-01` is marked as **ACCEPTED**.  
**RECOMMENDED NEXT TASK**: `P8-DIV-02` (Confirmed pivots and four-way divergence) per execution roadmap.
