# Independent Review Seal: Batch P3-VSA-02 — Simplified VSA, Volume Climax, and Candle Structure

**Date**: 2026-09-19  
**Batch ID**: `P3-VSA-02`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P3-VSA-02` expands the Sumi signal engine with modular, transparent Price+Volume technical inference signals (Volume Climax, VSA Strong/Weak Demand, Upthrust/Kéo Xả, Spring/Đạp Kéo, Support/Resistance composites, and Candle Structure Score) under strict technical-inference framing without unverified order-flow or institutional attribution claims (`SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 341–354, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections D.3, D.5, D.6, D.7).

### Key Architectural Accomplishments
1. **Technical Climax Identification (`SIG-VOL-003`)**:
   - `volume.climax_up` and `volume.climax_down` in `backend/app/domain/signals/volume.py`.
   - Combines relative volume ($\ge \text{multiplier}$), range expansion ($\text{range} / \text{ATR14} \ge 1.5$), and top/bottom quartile close location ($\ge 0.75$ for up, $\le 0.25$ for down).
2. **Transparent Candle Structure Scoring (`SIG-STR-001`)**:
   - `structure.candle_score` in `backend/app/domain/signals/structure.py`.
   - Linear combination of body direction, close location score ($2 \times \text{loc} - 1$), and wick balance ($(\text{lower\_wick} - \text{upper\_wick}) / \text{range}$) mapped deterministically to $[-100.0, +100.0]$.
   - Zero-range/flat candles yield exactly $0.0$ with quality `VALID` and `ZERO_RANGE_CANDLE` reason.
3. **Pure VSA Technical Primitives (`SIG-VSA-001..004`)**:
   - Implemented in `backend/app/domain/signals/vsa.py`:
     - `vsa.strong_demand`: Wide spread, RVOL $\ge 1.5$, body ratio $\ge 0.50$, close loc $\ge 0.70$, positive return.
     - `vsa.weak_demand`: Narrow spread ($\le 0.70$), low volume (RVOL $\le 0.80$), non-negative return.
     - `vsa.upthrust`: Intra-session breakout above prior resistance with high upper wick ($\ge 0.35$), weak close ($\le 0.45$), high RVOL ($\ge 1.5$).
     - `vsa.spring`: Intra-session breakdown below prior support with long lower wick ($\ge 0.35$), strong close ($\ge 0.70$), high RVOL ($\ge 1.30$).
4. **Causal Support/Resistance Composites (`SIG-VSA-005`, `SIG-VSA-006`)**:
   - `vsa.strong_demand_at_support`: `near_support AND (strong_demand OR spring)` with exact support level attribution (`ROLLING_LOW_20`, `EMA20`, `EMA50`).
   - `vsa.strong_supply_at_resistance`: `near_resistance AND (upthrust OR bearish_rejection)` with resistance level attribution.
5. **Causal Invariance (`TEST-CAUSAL-001`, `TEST-CAUSAL-003`)**:
   - Reference windows strictly exclude current bar $t$: `candles[t - period : t]`.
   - Appending future bars produces bit-for-bit identical historical outputs across all 9 new signals.
6. **Registry & AST Integration**:
   - All 9 signals registered in `SignalRegistry` with canonical parameters schemas and unique AST aliases (`vsa__*`, `structure__candle_score`, `volume__climax_*`). Evaluated and validated safely by `SignalBindingAdapter`.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `SIG-VOL-003` (Volume Climax Up/Down) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_volume_climax_up_and_down` validates RVOL, Range/ATR, and quartile close location. |
| `SIG-STR-001` (Candle Structure Score) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_candle_structure_score_and_bounds` confirms $[-100, 100]$ bounds and zero-range flat protection. |
| `SIG-VSA-001` (Strong Demand) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_strong_demand` validates spread, volume, body, and close thresholds. |
| `SIG-VSA-002` (Weak Demand) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_weak_demand` validates low volume and narrow spread criteria. |
| `SIG-VSA-003` (Upthrust / Kéo Xả) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_upthrust_keo_xa` confirms resistance breach, upper wick rejection, and weak close. |
| `SIG-VSA-004` (Spring / Đạp Kéo) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_spring_dap_keo` confirms support breach, lower wick rejection, and strong reclaim. |
| `SIG-VSA-005` (Demand at Support) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_strong_demand_at_support` validates support proximity and trigger attribution. |
| `SIG-VSA-006` (Supply at Resistance) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_strong_supply_at_resistance` validates resistance proximity and rejection trigger. |
| `TEST-CAUSAL-001` (Future Invariance) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_future_invariance` validates bit-for-bit identical outputs when future bars appended. |
| `TEST-CAUSAL-003` (Prior Reference Exclusion) | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_reference_window_causal_exclusion` ensures bar $t$ is excluded from resistance calculation. |
| AST & Registry Integration | `backend/app/tests/test_vsa_signals.py` | **PASS** | `test_vsa_registry_and_binding` verifies registry dispatch, parameter validation, and AST rule evaluation. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Baseline hash strictly matches: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved untouched. |

---

## 3. Automated Test Evidence

### Backend Focused Signal Suite
```powershell
$env:PYTHONPATH="backend"; python -m pytest backend/app/tests/test_signals.py backend/app/tests/test_signal_binding.py backend/app/tests/test_signals_api.py backend/app/tests/test_price_signals.py backend/app/tests/test_vsa_signals.py -v
```
- **Result**: 63 passed in 0.47s.

### Fast Technical Gate (`.\scripts\verify-v2.ps1`)
- **Backend Tests**: 349 passed (0 failed).
- **Alembic Migrations**: Upgraded successfully on temporary database.
- **Frontend Linter**: Clean (`eslint .` passed).
- **Frontend Tests**: 32 test files, 210 passed (0 failed).
- **Frontend Production Build**: `tsc -b && vite build` built successfully in 668ms.

### Comprehensive Browser E2E UAT (`.\scripts\run-comprehensive-uat.ps1`)
- **Total Scenarios**: 31/31 passed (100.0%).
- **Runtime Console Errors**: 0 errors (`TC-SYS-04: PASS`).
- **Database Immutability**: Verified (`TC-SYS-05: PASS`).

---

## 4. Reviewer Seal & Decision

Batch `P3-VSA-02` fulfills all functional and technical acceptance requirements specified in `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 341–354 and `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md`.

**DECISION: ACCEPTED AND SEALED**

**RECOMMENDED NEXT TASK**: `P8-ICHI-01` (Causal Ichimoku cloud and TK cross) or `P8-DIV-02` (Multi-indicator divergence) per dependency readiness in `STATE.json`.
