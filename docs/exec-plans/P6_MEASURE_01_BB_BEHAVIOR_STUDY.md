# P6-MEASURE-01 — BB Behavior Measurement Study on Real Audited Sample

## Outcome
A comprehensive, reproducible measurement study and empirical evidence report analyzing the statistical behavior of Symbol BB (`OHLCV_PROXY`) across standard horizons (`T03`, `T05`, `T10`, `T20`, `T50`, `T200`) using 673 sessions of real audited market data (`FPT`, `HPG`, `SSI`) from `2024-01-02` to `2026-09-18`. Delivers machine-readable JSON results (`docs/research/bb_measurement_report_P6_01.json`) and a detailed analytical findings document (`docs/research/BB_BEHAVIOR_MEASUREMENT_P6_01.md`) covering distributions, zone occupation, smoothness, persistence, cross-horizon dynamics, and empirical evaluation of 20/30/70/80 threshold candidates. Strictly adheres to the evidence-first principle: no P&L optimization, no hardcoded production threshold semantics ("not production semantics yet"), and zero database mutation.

## Context and problem
- **Addressed requirements:** `BBI-SIG-002`, `FR-CORE-012`, `NFR-DET-001`, `TEST-BB-002`, BB Spec Section 16 (`SUMI_Money_Flow_Blackbox_V1_Handoff`).
- **Context:** `P5-DATA-01` established data contracts and `P5-BB-02` implemented the pure `ProxyBBCalculator`.
- **Problem:** Technical analysis practitioners often naively borrow RSI-style 20/30/70/80 or 30/70 thresholds for proprietary flow indicators without empirical measurement. Before freezing events (`TURN_UP`, `TURN_DOWN`, `CONFLUENCE`) or trading labels, an authoritative measurement study must determine what ProxyBB lines actually measure, their distributions, lead/lag relationships, and whether fixed thresholds are statistically justified.

## In scope
1. **Audited Data Ingestion & Preservation**:
   - Target freshness check executed on Doraemon production API (`2024-01-01` to `2026-09-18`) for `FPT`, `HPG`, `SSI` (673 daily sessions per symbol).
   - Deterministic, versioned fixture archive stored in `backend/app/tests/fixtures/real_audited_sample_fpt_hpg_ssi.json` for reproducible offline verification.
2. **Measurement Engine & Research Runner** (`backend/scripts/measure_bb_behavior.py`):
   - Computes multi-horizon BB series for all sample tickers.
   - Calculates statistical distributions: mean, standard deviation, median, min, max, IQR, 5th, 25th, 75th, 95th percentiles by horizon.
   - Evaluates zone occupancy time: $\% \le 20$, $\% \le 30$, $\% \in (30, 70)$, $\% \ge 70$, $\% \ge 80$.
   - Measures smoothness & stability: daily delta stats ($\Delta BB$), lag-1 autocorrelation, saturation rates ($BB \in \{0, 100\}$), and 50-line crossing frequency.
   - Evaluates cross-horizon dynamics: confluence rates, lead/lag relationships ($T03 \to T05 \to T20 \to T50 \to T200$), and regime persistence / run lengths.
   - Segments analysis by value source (`ACTUAL_MATCHED_VALUE` vs `ESTIMATED_TP_X_VOLUME`).
3. **Evidence Artifacts**:
   - Machine-readable output: `docs/research/bb_measurement_report_P6_01.json`.
   - Comprehensive analytical report: `docs/research/BB_BEHAVIOR_MEASUREMENT_P6_01.md`.
4. **Validation Test Suite** (`backend/app/tests/test_bb_measurement.py`):
   - Automated test verifying measurement computation accuracy, percentile monotonicity, zone sum consistency, and reproducibility.

## Out of scope
- P&L backtest optimization to find profitable parameters.
- Hardcoding production trading signals on 20/30/70/80 thresholds.
- Promoting Market BB / whole-market breadth (deferred to Phase 7).
- Database mutations on `backend/sumi.db`.

## Invariants
- **No P&L selection bias:** Analysis evaluates measurement distributions, never trade profitability.
- **Future invariance:** Calculations use strictly causal historical windows.
- **Zero database mutation:** Does not write to `backend/sumi.db`.
- **Git hygiene:** 159 historically staged deletions preserved.

## Milestones
1. [x] **Milestone 1**: Execute targeted freshness check and persist audited 673-session multi-symbol fixture in `backend/app/tests/fixtures/real_audited_sample_fpt_hpg_ssi.json`.
2. [x] **Milestone 2**: Implement research measurement runner in `backend/scripts/measure_bb_behavior.py`.
3. [x] **Milestone 3**: Generate machine-readable report `docs/research/bb_measurement_report_P6_01.json` and comprehensive analytical report `docs/research/BB_BEHAVIOR_MEASUREMENT_P6_01.md`.
4. [x] **Milestone 4**: Implement test suite `backend/app/tests/test_bb_measurement.py` verifying measurement metrics.
5. [x] **Milestone 5**: Execute fast technical gate (`verify-v2.ps1`) and comprehensive browser UAT (`run-comprehensive-uat.ps1`).
6. [x] **Milestone 6**: Independent review seal, ExecPlan completion, and `STATE.json` update.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `BBI-SIG-002` | Empirical threshold evaluation | `BB_BEHAVIOR_MEASUREMENT_P6_01.md` / `test_bb_measurement.py` |
| `FR-CORE-012` | BB calculation on audited data | `test_bb_measurement.py::test_bb_measurement_reproducibility` |
| `NFR-DET-001` | Bit-for-bit deterministic report | `test_bb_measurement.py::test_measurement_determinism` |
| `TEST-BB-002` | Guardrail: "not production semantics yet" | `BB_BEHAVIOR_MEASUREMENT_P6_01.md` Section 6 & conclusion |

## Verification commands
```powershell
# 1. Measurement runner
.\.venv\Scripts\python.exe scripts/measure_bb_behavior.py

# 2. Measurement test suite
.\.venv\Scripts\python.exe -m pytest app/tests/test_bb_measurement.py -v

# 3. Fast technical gate
.\scripts\verify-v2.ps1

# 4. Comprehensive browser UAT
.\scripts\run-comprehensive-uat.ps1
```

## Rollback and compatibility
`P6-MEASURE-01` is strictly an empirical research and measurement batch. It adds research scripts, fixture data, and research reports. No database migrations, runtime models, or API contracts are altered.

## Progress log
- 2026-09-18: ExecPlan created for P6-MEASURE-01.
- 2026-09-18: Targeted freshness check completed for Doraemon API (`FPT`, `HPG`, `SSI` from `2024-01-01` to `2026-09-18`, 673 sessions). Logged in `DORAEMON_MARKET_DATA_AUDIT.md`.
- 2026-09-18: Persisted audited fixture to `backend/app/tests/fixtures/real_audited_sample_fpt_hpg_ssi.json` (673 daily sessions per symbol).
- 2026-09-18: Implemented `backend/scripts/measure_bb_behavior.py` and generated `docs/research/bb_measurement_report_P6_01.json` and `docs/research/BB_BEHAVIOR_MEASUREMENT_P6_01.md`.
- 2026-09-18: Formally issued research verdict: `NOT_PRODUCTION_SEMANTICS_YET` on 20/30/70/80 thresholds based on empirical extreme corridor compression (95.16% on T20, 99.15% on T50).
- 2026-09-18: Implemented test suite `backend/app/tests/test_bb_measurement.py` (4/4 passed).
- 2026-09-18: Verified fast technical gate `verify-v2.ps1` (323 backend tests passed, Alembic migrations passed, frontend lint passed, Vitest 210 tests passed, Vite build passed).
- 2026-09-18: Verified comprehensive browser UAT `run-comprehensive-uat.ps1` (31/31 scenarios passed, 100%).
- 2026-09-18: Verified database integrity: `backend/sumi.db` SHA-256 hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (zero mutations).
- 2026-09-18: Milestone sealed with independent review `docs/reviews/P6_MEASURE_01_REVIEW.md`.
