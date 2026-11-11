# Sumi V3 Release: Acceptance Matrix and Product Limitations

**Document Version**: 3.0.0  
**Date**: 2026-09-19  
**Status**: Authoritative Reference Documentation  
**Product Standard**: Local-first manual replay, research validation, and backtesting for Vietnam equity markets (`PRODUCT_ACCEPTANCE_CRITERIA_V3.md`).

---

## 1. Traceability Acceptance Matrix

| Batch ID | Capability Area | Specific Acceptance IDs | Implemented Core Modules | Verification Evidence | Status |
|---|---|---|---|---|---|
| **BOOTSTRAP** | Workspace Baseline | Program baseline freeze, causal harness, critic auditor | `docs/dev-program/`, `verify-v2.ps1` | `docs/reviews/DEV_PROGRAM_BOOTSTRAP_EVIDENCE.md` | **ACCEPTED** |
| **P1-SIG-01 / 02** | Canonical Signals | `SIG-001`, `SIG-002`, `TEST-PARAM-001`, `TEST-SERIAL-001` | `app.domain.signals.models`, `app.domain.signals.volume` | `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md` | **ACCEPTED** |
| **P2-BT-01** | Next-Event Backtest Kernel | `BT-EXEC-001`, `BT-ORD-001`, `BT-FILL-001`, `TEST-EXEC-001` | `app.domain.backtest.kernel`, `app.domain.backtest.models` | `docs/reviews/P2_BT_01_REVIEW.md` | **ACCEPTED** |
| **P2-MKT-02** | Vietnam Market Rules | `MKT-VN-001`, `MKT-VN-002`, `MKT-LOT-001`, `MKT-FEE-001` | `app.domain.market.rules`, `app.domain.market.settlement` | `docs/reviews/P2_MKT_02_REVIEW.md` | **ACCEPTED** |
| **P3-PRICE-01** | Price Signals & Regimes | `SIG-PAT-001`, `SIG-STR-001`, `SIG-REG-001`, `SIG-TRIG-001` | `app.domain.signals.patterns`, `app.domain.signals.regimes` | `docs/reviews/P3_PRICE_01_REVIEW.md` | **ACCEPTED** |
| **P3-VSA-02** | Volume Spread Analysis | `SIG-VSA-001`, `SIG-VSA-002`, `SIG-SR-001` | `app.domain.signals.vsa`, `app.domain.signals.support_resistance` | `docs/reviews/P3_VSA_02_REVIEW.md` | **ACCEPTED** |
| **P4-BATCH-01** | Multi-Phase Batch Runner | `FR-CORE-010`, `FR-CORE-011`, `NFR-PERF-001` | `app.domain.backtest.batch_runner`, `app.services.backtest_service` | `docs/reviews/P4_BATCH_01_REVIEW.md` | **ACCEPTED** |
| **P4-MET-02** | Metric Matrix & Degradation | `FR-CORE-012`, `TEST-MET-001`, `F.5 Guardrails` | `app.domain.backtest.benchmark_metrics`, `batch_runner.py` | `docs/reviews/P4_MET_02_REVIEW.md` | **ACCEPTED** |
| **P5-DATA-01** | Market Data Contracts | `DATA-PORT-001`, `DATA-SCHEMA-001`, `DATA-SANITY-001` | `app.domain.data.ports`, `app.domain.data.contracts` | `docs/reviews/P5_DATA_01_REVIEW.md` | **ACCEPTED** |
| **P5-BB-02** | Symbol Money Flow BB | `SIG-BB-001`, `SIG-BB-002`, `TEST-BB-001` | `app.domain.bb.calculator`, `app.services.bb_service` | `docs/reviews/P5_BB_02_REVIEW.md` | **ACCEPTED** |
| **P6-MEASURE-01** | BB Empirical Measurement | `STUDY-BB-001`, `TEST-REPRO-001`, Master H.1 | `backend/scripts/measure_bb_behavior.py` | `docs/reviews/P6_MEASURE_01_REVIEW.md` | **ACCEPTED** |
| **P7-UNI-01** | SUMI-420 Universe | `UNI-420-001`, `UNI-GOV-001`, `TEST-UNI-001` | `app.domain.universe.models`, `generate_sumi420_candidate.py` | `docs/reviews/P7_UNI_01_REVIEW.md` | **ACCEPTED** |
| **P7-MKT-02** | Market BB Breadth Gate | `MKT-BB-001`, `MKT-BREADTH-001`, `TEST-MKT-001` | `app.domain.bb.market_bb`, `app.api.bb` | `docs/reviews/P7_MKT_02_REVIEW.md` | **ACCEPTED** |
| **P8-ICHI-01** | Causal Ichimoku Signals | `SIG-ICHI-001`, `SIG-ICHI-002`, `TEST-CAUSAL-001` | `app.domain.signals.ichimoku` | `docs/reviews/P8_ICHI_01_REVIEW.md` | **ACCEPTED** |
| **P8-DIV-02** | Confirmed Divergences | `SIG-DIV-001`, `SIG-DIV-002`, `TEST-DELAY-001` | `app.domain.signals.divergence` | `docs/reviews/P8_DIV_02_REVIEW.md` | **ACCEPTED** |
| **P8-COMP-03** | Health & BB Events | `SIG-HEALTH-001`, `SIG-FLOW-001`, `SIG-BIND-001` | `app.domain.signals.health`, `app.domain.signals.flow_events` | `docs/reviews/P8_COMP_03_REVIEW.md` | **ACCEPTED** |
| **P9-UI-01** | Unified Research Surface | `UI-SIG-001..003`, `UI-EXP-001`, `FR-CORE-006` | `frontend/src/components/signals/`, `components/strategy/` | `docs/reviews/P9_UI_01_REVIEW.md` | **ACCEPTED** |
| **P10-PERF-01** | Verified Hotspots Cache | `NFR-PERF-001/002`, `TEST-PERF-001`, `TEST-REPRO-001` | `app.domain.engine.cache`, `benchmark_research_hotspots.py` | `docs/reviews/P10_PERF_01_REVIEW.md` | **ACCEPTED** |

---

## 2. Invariant Commitments

1. **Zero Future Candle Leak**:
   - Replay endpoints, indicator calculations, signal registries, and backtest loops operate strictly upon prefixes ending at `current_index` or the designated historical event date. Slicing in the browser or pre-fetching future candles is strictly forbidden.
2. **Authoritative Determinism (`TEST-REPRO-001`)**:
   - Running backtests or signal calculations with caching enabled vs. disabled produces bit-for-bit identical numerical results across cash, equity, trade logs, and metrics.
3. **Database Immutability**:
   - The reference database `backend/sumi.db` is strictly treated as read-only. All automated tests and browser UAT sessions execute against temporary in-memory or ephemeral databases. Hash baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` is verified continuously.
4. **Local-First Privacy**:
   - Zero telemetry, zero external tracking, and zero sending of user trading journals or market notes outside the local machine.

---

## 3. Explicit Product Scope and Limitations

### 3.1 Money Flow Bollinger Bands (`OHLCV_PROXY`)
- **Methodology Boundary**: Sumi Money Flow BB metrics across `T03`–`T200` are computed using an institutional `OHLCV_PROXY` algorithm derived from price-range location and volume distribution.
- **Explicit Research Disclaimer**: These flow metrics are empirical estimates designed for technical confluence and regime filtering. They do not represent raw tick-by-tick order book aggressive match data.
- **Future Upgrade Path**: When commercial exchange feeds (e.g. FiinQuant or HOSE proprietary tick feeds) are procured, raw classified flow can be integrated via the existing `MarketDataProvider` port without changing the signal interface.

### 3.2 Vietnam Market Rules
- **Settlement**: Causal modeling of Vietnam T+2 / T+2.5 settlement cycle, 100-share lot size restrictions, and exchange price step constraints (HOSE/HNX).
- **Execution Basis**: Orders execute on the next candle's open or next valid tick price; same-candle close execution is prohibited.

### 3.3 Trading Practice & Execution
- **Intended Purpose**: Sumi is a rigorous technical analysis, manual replay practice, and algorithmic backtesting platform.
- **Live Trading Limitation**: Sumi does not provide direct live broker DMA (Direct Market Access) order routing or API trading execution. Real trading must be executed through approved broker terminals.
