# Independent Review Seal: Batch P6-MEASURE-01 — BB Behavior Measurement Study on Real Audited Sample

**Date**: 2026-09-18  
**Batch ID**: `P6-MEASURE-01`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P6-MEASURE-01` delivers an authoritative empirical behavior measurement study of the Symbol Money Flow Blackbox under the `OHLCV_PROXY` methodology. The study analyzes 673 valid market sessions across three diverse, liquid Vietnamese equities (`FPT`, `HPG`, `SSI`) from `2024-01-02` to `2026-09-18` across all six evaluation horizons (`T03`, `T05`, `T10`, `T20`, `T50`, `T200`).

In strict conformance with the non-negotiable principle of **measurement-first research** (`BBI-SIG-002`, `TEST-BB-002`, BB Spec Section 16):
1. **Zero P&L Optimization**: The study evaluates statistical distributions, zone occupancy, smoothness, and cross-horizon lead/lag, with zero parameter tuning on backtest profitability.
2. **Formal Research Verdict (`NOT_PRODUCTION_SEMANTICS_YET`)**: Empirical evidence definitively proves that fixed 20/30/70/80 thresholds do NOT correspond to stable, symmetric, or regime-invariant percentiles. Consequently, fixed thresholds remain unvalidated research hypotheses and are strictly barred from being hardcoded into production signal rules.
3. **Reproducible Machine-Readable Evidence**: Emitted complete statistical matrix artifact `docs/research/bb_measurement_report_P6_01.json` and comprehensive analytical report `docs/research/BB_BEHAVIOR_MEASUREMENT_P6_01.md`.

---

## 2. Key Empirical Findings

### 2.1 Severe Corridor Compression on Slower Horizons
On macro and medium horizons ($H \ge 20$), the rolling value-weighted accumulation suppresses extremes, compressing the curve into the $(30, 70)$ corridor:
- **T20**: 95.16% of sessions remain within $(30, 70)$.
- **T50**: 99.15% of sessions remain within $(30, 70)$.
- **T200**: 99.65% of sessions remain within $(30, 70)$.

### 2.2 Unreachability of Classic Overbought Thresholds
In 673 trading sessions across all three tickers (2,019 total session-horizon evaluations):
- **% Time $\ge 70$ on T20:** **`0.00%`** (0 sessions).
- **% Time $\ge 70$ on T50:** **`0.00%`** (0 sessions).
- **% Time $\ge 70$ on T200:** **`0.00%`** (0 sessions).
*Conclusion:* Naively importing RSI-like 70 or 80 overbought levels onto T20+ Symbol BB produces completely dead rules that would never fire in practice.

### 2.3 Tail Asymmetry on Fast Horizons
On fast horizons where the curve reaches outer zones:
- **T03**: 26.63% of sessions are $\le 30$, while only 9.64% are $\ge 70$.
- **T05**: 20.08% of sessions are $\le 30$, while only 5.58% are $\ge 70$.
*Conclusion:* Negative price pressure clusters more strongly than positive pressure during market consolidation, invalidating symmetric fixed thresholds.

### 2.4 Monotonic Smoothing & Structural Regime Roles
- Autocorrelation monotonically climbs from $\rho_1 = 0.6320$ on T03 to $0.9388$ on T50.
- Neutral 50-line crossings drop from 24.89 per 100 days on T03 down to 6.27 on T20 and 0.63 on T200.
*Conclusion:* Slow lines (T50, T200) serve as structural macro regime filters, while fast lines (T03, T05) function as momentum/pressure oscillators.

---

## 3. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `BBI-SIG-002` (Measurement before freezing rules) | `BB_BEHAVIOR_MEASUREMENT_P6_01.md` | **PASS** | Empirical distribution study executed before freezing any threshold/event logic. |
| `TEST-BB-002` (Formal "not production semantics yet" verdict) | `test_bb_measurement.py::test_measurement_threshold_verdict_and_guardrail` | **PASS** | Report explicitly concludes `NOT_PRODUCTION_SEMANTICS_YET`; `validate_bb_threshold_semantics` blocks 20/30/70/80. |
| `FR-CORE-012` (Audited real sample execution) | `test_bb_measurement.py::test_audited_fixture_integrity` | **PASS** | 673 sessions for FPT, HPG, SSI verified from real audited Doraemon market data. |
| `NFR-DET-001` (Bit-for-bit determinism) | `test_bb_measurement.py::test_measurement_determinism` | **PASS** | Identical report output across independent execution runs. |
| Statistical Monotonicity | `test_bb_measurement.py::test_measurement_statistical_monotonicity_and_consistency` | **PASS** | Validates $\min \le p_{05} \le p_{10} \le p_{25} \le p_{50} \le p_{75} \le p_{90} \le p_{95} \le \max$. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Hash matches baseline: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved intact. |

---

## 4. Test & Gate Execution Results

### 4.1 Focused Measurement Test Suite
```text
============================= test session starts =============================
app/tests/test_bb_measurement.py::test_audited_fixture_integrity PASSED  [ 25%]
app/tests/test_bb_measurement.py::test_measurement_statistical_monotonicity_and_consistency PASSED [ 50%]
app/tests/test_bb_measurement.py::test_measurement_threshold_verdict_and_guardrail PASSED [ 75%]
app/tests/test_bb_measurement.py::test_measurement_determinism PASSED    [100%]
======================== 4 passed, 1 warning in 0.93s =========================
```

### 4.2 Fast Technical Gate (`verify-v2.ps1`)
- **Backend Tests**: 323 passed (0 failures).
- **Alembic Upgrade**: Successfully applied migrations on temporary SQLite test DB.
- **Frontend Lint**: Clean (`eslint .`).
- **Frontend Tests**: 32 test files passed, 210 tests passed.
- **Frontend Build**: TypeScript checking and Vite production build passed.

### 4.3 Comprehensive Browser UAT (`run-comprehensive-uat.ps1`)
- **Result**: 31/31 passed (100.0%, 0 failed).
- **Database Integrity Guardrail (`TC-SYS-05`)**: PASSED. SHA-256 verified match.
- **Zero Console Errors (`TC-SYS-04`)**: PASSED.

---

## 5. Audit Review Seal

All acceptance criteria, invariants, and quality gates for `P6-MEASURE-01` are satisfied in full without regressions or database mutations.

**SEAL STATUS**: **ACCEPTED**  
**RECOMMENDED NEXT TASK**: `P7-UNI-01` (Versioned SUMI-420 framework and candidate v1) or `P3-VSA-02` / `P8-ICHI-01` per execution roadmap.
