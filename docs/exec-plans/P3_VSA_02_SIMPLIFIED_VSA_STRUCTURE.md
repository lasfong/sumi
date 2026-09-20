# P3-VSA-02 — Simplified VSA, Volume Climax, and Candle Structure

## Outcome
Deliver modular, transparent Price+Volume technical inference signals (Volume Climax, VSA Strong/Weak Demand, Upthrust/Kéo Xả, Spring/Đạp Kéo, Demand/Supply at Support/Resistance composites, and Candle Structure Score) into the Sumi signal engine. Ensure strictly causal evaluation with zero future lookahead, zero-range candle safety, full source/metric reason attribution, and strictly technical labeling without claiming observed institutional identity or unverified order-flow distribution.

## Context and problem
- **Canonical References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 341–354, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections D.3, D.5, D.6, D.7; `AGENTS.md`.
- **Problem**:
  1. Technical analysis indicators frequently brand price-volume patterns as "institutional accumulation", "smart money dumping", or "guaranteed supply/demand zones" without empirical validation or order-book telemetry.
  2. Pattern and VSA algorithms often leak future bars by including the current bar $t$ in historical reference extrema (rolling high/low) or using centered moving averages.
  3. Zero-range / flat trading sessions (e.g., limit-lock floor/ceiling or zero liquidity) cause zero-division crashes in candle geometry unless guarded with deterministic fallbacks.
- **Acceptance IDs**:
  - `SIG-VOL-003`: Volume Climax Up and Volume Climax Down based on relative volume, range/ATR, and close location.
  - `SIG-VSA-001`: Strong Demand (wide spread, above-average RVOL, top close location, positive return).
  - `SIG-VSA-002`: Weak Demand / No Demand (narrow spread, below-average RVOL, positive or flat return).
  - `SIG-VSA-003`: Upthrust / Simplified Kéo Xả (temporary breach of prior resistance with high upper wick, weak close, high relative volume).
  - `SIG-VSA-004`: Spring / Shakeout / Đạp Kéo (temporary breach of prior support with long lower wick, strong close, high relative volume).
  - `SIG-VSA-005`: Strong Demand at Support composite (`near_support AND (strong_demand OR spring)`).
  - `SIG-VSA-006`: Strong Supply at Resistance composite (`near_resistance AND (upthrust OR strong_bearish_rejection)`).
  - `SIG-STR-001`: Candle Structure Score in `[-100.0, 100.0]` integrating direction, body ratio, close location, and wick balance.
  - `TEST-CAUSAL-001`: Future appending invariance (adding future bars does not mutate historical signal values).
  - `TEST-CAUSAL-003`: Prior reference exclusion (bar $t$ strictly excluded from historical rolling support/resistance windows).

## In scope
- Module `backend/app/domain/signals/vsa.py`: Core pure calculation functions for `vsa.strong_demand`, `vsa.weak_demand`, `vsa.upthrust`, `vsa.spring`, `vsa.strong_demand_at_support`, and `vsa.strong_supply_at_resistance`.
- Module `backend/app/domain/signals/structure.py`: Core calculation function for `structure.candle_score` (`SIG-STR-001`).
- Addition to `backend/app/domain/signals/volume.py`: Pure calculation functions for `volume.climax_up` and `volume.climax_down` (`SIG-VOL-003`).
- Updates to `backend/app/domain/signals/registry.py`: Register all 9 new signal definitions, schemas, defaults, and AST aliases (`vsa__*`, `volume__climax_*`, `structure__candle_score`).
- Test suite `backend/app/tests/test_vsa_signals.py`: Verification of each formula, boundary edge, zero-range candle handling, reason attribution, and future invariance.

## Out of scope
- Multi-indicator divergence signals (`P8-DIV-02`).
- Ichimoku cloud & TK cross (`P8-ICHI-01`).
- Multi-signal composite health scoring (`P8-COMP-03`).
- Modifying database schemas or tables in `backend/sumi.db`.

## Invariants
- **No Future Leakage**: Reference resistance/support extrema use `candles[t-period : t]`, strictly excluding index $t$.
- **Causal ATR**: ATR calculated causally via `calculate_causal_atr` with RMA smoothing.
- **Zero-Range Safety**: Flat bars with `range <= 1e-8` produce neutral score and no exceptions.
- **Strictly Technical Labeling**: Documentation and reason strings describe price/volume geometry without claiming institutional intent.
- **Preserved Repository State**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` and 159 staged deletions remain untouched.

## Current architecture
- `backend/app/domain/signals/`: Contains `volume.py`, `candle_features.py`, `patterns.py`, `regimes.py`, `support_resistance.py`, `technical.py`, and `registry.py`.
- `backend/app/domain/strategy/signal_binding.py`: AST alias resolver and safe evaluator consuming `SignalRegistry`.

## Target design
- Pure domain calculation modules:
  - `backend/app/domain/signals/vsa.py`
  - `backend/app/domain/signals/structure.py`
  - Enhanced `backend/app/domain/signals/volume.py`
- Dispatch and metadata registered in `SignalRegistry`.

## Milestones
1. **Milestone 1**: Implement `volume.climax_up` and `volume.climax_down` in `volume.py` with validation and tests.
2. **Milestone 2**: Implement `structure.candle_score` in `structure.py` with transparent formula and boundary tests.
3. **Milestone 3**: Implement pure VSA primitives (`vsa.strong_demand`, `vsa.weak_demand`, `vsa.upthrust`, `vsa.spring`, composites at support/resistance) in `vsa.py`.
4. **Milestone 4**: Wire all signals into `SignalRegistry`, update registry exports and AST aliases.
5. **Milestone 5**: Full test coverage in `backend/app/tests/test_vsa_signals.py`, full gates (`verify-v2.ps1`, `run-comprehensive-uat.ps1`), independent review seal.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
|---|---|---|
| `SIG-VOL-003` | `backend/app/domain/signals/volume.py` (`climax_up`, `climax_down`) | `test_vsa_signals.py::test_volume_climax_up_and_down` |
| `SIG-VSA-001` | `backend/app/domain/signals/vsa.py` (`strong_demand`) | `test_vsa_signals.py::test_vsa_strong_demand` |
| `SIG-VSA-002` | `backend/app/domain/signals/vsa.py` (`weak_demand`) | `test_vsa_signals.py::test_vsa_weak_demand` |
| `SIG-VSA-003` | `backend/app/domain/signals/vsa.py` (`upthrust`) | `test_vsa_signals.py::test_vsa_upthrust_keo_xa` |
| `SIG-VSA-004` | `backend/app/domain/signals/vsa.py` (`spring`) | `test_vsa_signals.py::test_vsa_spring_dap_keo` |
| `SIG-VSA-005` | `backend/app/domain/signals/vsa.py` (`strong_demand_at_support`) | `test_vsa_signals.py::test_vsa_strong_demand_at_support` |
| `SIG-VSA-006` | `backend/app/domain/signals/vsa.py` (`strong_supply_at_resistance`) | `test_vsa_signals.py::test_vsa_strong_supply_at_resistance` |
| `SIG-STR-001` | `backend/app/domain/signals/structure.py` (`candle_score`) | `test_vsa_signals.py::test_candle_structure_score_and_bounds` |
| `TEST-CAUSAL-001` | Pure causal implementation | `test_vsa_signals.py::test_vsa_future_invariance` |
| `TEST-CAUSAL-003` | Reference exclusion $t-N..t$ | `test_vsa_signals.py::test_vsa_reference_window_causal_exclusion` |

## Verification commands
```powershell
python -m pytest backend/app/tests/test_vsa_signals.py -v
.\scripts\verify-v2.ps1
.\scripts\run-comprehensive-uat.ps1
Get-FileHash "backend\sumi.db" -Algorithm SHA256
node scripts/verify-dev-program.mjs
```

## Rollback and compatibility
- Revert additions to `backend/app/domain/signals/` and `SignalRegistry`.
- No database migrations exist, leaving `backend/sumi.db` intact.

## Risks and mitigations
- **Risk**: Semantic overclaiming of VSA patterns.
  - **Mitigation**: Signal names, labels, and docstrings strictly describe price/volume geometry; no claims of smart money or institutional manipulation.
- **Risk**: Zero-range candles causing division by zero.
  - **Mitigation**: Reuse `calculate_candle_geometry` with zero-range protections.
- **Risk**: Future lookahead in support/resistance breach detection.
  - **Mitigation**: Strict slicing `candles[t-period : t]` excluding current bar $t$.

## Progress log
- 2026-09-19: ExecPlan created for batch `P3-VSA-02`. Implementation commenced.
- 2026-09-19: Implemented `volume.climax_up`, `volume.climax_down`, `structure.candle_score`, and VSA primitives (`vsa.strong_demand`, `vsa.weak_demand`, `vsa.upthrust`, `vsa.spring`, `vsa.strong_demand_at_support`, `vsa.strong_supply_at_resistance`). Registered in `SignalRegistry` with clean AST aliases. Added 11 focused tests in `test_vsa_signals.py` (63 total signal tests). All technical and browser UAT gates passed. Independent review sealed.

## Decision log
- **Decision**: Separate VSA primitives into dedicated `vsa.py` and `structure.py` domain modules rather than overloading `volume.py` or `patterns.py`.
  - **Rationale**: Keeps cohesion high, follows modular domain architecture, and cleanly isolates structure scoring and composite logic.
- **Decision**: Use ATR-normalized buffer for Upthrust and Spring breach calculations.
  - **Rationale**: Direct compliance with Master Spec section D.5.

## Completion evidence
- Review Seal: `docs/reviews/P3_VSA_02_REVIEW.md` (ACCEPTED).
- Unit Tests: `backend/app/tests/test_vsa_signals.py` (11/11 passed, 63/63 signal tests passed).
- Fast Technical Gate: `.\scripts\verify-v2.ps1` (349 backend tests, 210 frontend tests, ESLint, Vite build - 0 failures).
- Comprehensive Browser UAT: `.\scripts\run-comprehensive-uat.ps1` (31/31 scenarios passed, 100.0%, zero console errors, zero DB mutation).
- Database Immutability: SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` verified unchanged.
- Preserved Staged Deletions: 159 historically staged deletions preserved intact.

