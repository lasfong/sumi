# P7-MKT-02 — Market BB/Breadth framework and publication gate

## Outcome
Deliver the pure Market Aggregator and Breadth calculation engine implementing value-weighted aggregate-before-ratio money flow, independent flow breadth (count and value distributions), and publication guardrails. Enforce that canonical publishing remains disabled unless all dated universe/method/coverage gates pass. If evaluated on retrospective snapshots or candidate universes, output is strictly labeled `RESEARCH_RETROSPECTIVE` with explicit survivorship caveats; if coverage fails (<50%), output is `UNAVAILABLE_DEGRADED`.

## Context and problem
- **Canonical References**: `Master G.4 / Section 26`, `BB v3 08`, `BBI-SIG-001/003`, `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 447–460, 563, 629, 655, 711–736.
- **Problem**:
  1. Averaging individual symbol BB scores (`mean(Symbol BB)`) severely distorts whole-market money flow because it ignores trading value disparities across large-cap and small-cap equities.
  2. Technical analysis products frequently substitute index volume (e.g., VNINDEX volume) as "market money flow", which fails to reflect member money flow direction and aggressor pressure.
  3. Running whole-market metrics against unvalidated candidate universes or across historical periods with low constituent coverage produces misleading macro signals unless governed by strict publication gates and retrospective caveats.
- **Acceptance IDs**:
  - `BB-MKT-001`: Aggregate-before-ratio identity (`AT09`): Market BB equals the ratio of universe-aggregated buy value to universe-aggregated total trading value over rolling horizon $H$.
  - `BB-MKT-002`: No score averaging (`AT10`): Constructed mathematical proof demonstrating that $\text{mean}(BB_i) \ne \text{MarketBB}$.
  - `BB-MKT-003`: Universe point-in-time boundary (`AT11`): Member additions and delistings contribute only during active eligibility intervals.
  - `BB-MKT-004`: Independent Flow Breadth (`AT17`): Positive, neutral, and negative count and value breadths are computed independently from Market BB, and sum to 1.0 within tolerance.
  - `BB-MKT-005`: Publication gate & retrospective caveat: Canonical publication is blocked for candidate/retrospective universes and low coverage (<50%), emitting explicit `RESEARCH_RETROSPECTIVE` or `UNAVAILABLE_DEGRADED` states.

## In scope
- Contracts in `backend/app/domain/bb/contracts.py`:
  - `MarketPublicationStatus` (`CANONICAL_PUBLISHED`, `RESEARCH_RETROSPECTIVE`, `UNAVAILABLE_DEGRADED`).
  - `MarketBBHorizonPoint` (aggregated buy, sell, total value, Market BB, direction, regime, warmup).
  - `FlowBreadthHorizonPoint` (positive, neutral, negative count and value sums and ratios, buffer).
  - `MarketBBSessionPoint` (composite daily session point with horizons, breadth, and coverage).
  - `MarketBBSeriesResult` (complete series response with publication status, universe metadata, coverage report, and caveats).
- Pure calculation engine in `backend/app/domain/bb/market_aggregator.py`:
  - `MarketBBAggregator.calculate_market_bb`: Pure domain aggregator computing daily active buy/sell values, rolling horizon ratios, independent breadth distributions, and coverage metrics.
  - `MarketBBAggregator.evaluate_publication_gate`: Pure gate validator determining publication status.
- Pydantic schemas in `backend/app/schemas/bb_schema.py`:
  - `MarketBBCalculateRequest`, `MarketBBHorizonPointResponse`, `FlowBreadthResponse`, `MarketBBSessionPointResponse`, `MarketBBSeriesResponse`.
- Service extension in `backend/app/services/bb_service.py`:
  - `calculate_market_bb` integrating `UniverseResolver`, `UniverseRegistry`, and `MarketBBAggregator`.
- API router in `backend/app/api/bb.py`:
  - `POST /api/bb/market/calculate`
  - `GET /api/bb/market/{universe_id}`
- Test suite in `backend/app/tests/test_market_bb.py`:
  - Unit and fixture tests verifying AT09, AT10, AT11, AT12, AT17, and publication gate rules.

## Out of scope
- Mutating or adding tables to `backend/sumi.db` (no `bb_market_daily` or `bb_breadth_daily` SQLite tables in Sumi MVP).
- Promoting `SUMI420_v1_candidate` to canonical `SUMI420_v1` without owner approval.
- Displaying VNINDEX raw index volume as Market BB.
- Hardcoding production trading rules on 20/30/70/80 thresholds.

## Invariants
- `backend/sumi.db` SHA-256 hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` must remain untouched.
- Single production writer on branch `master`.
- No historical staged deletions touched.
- AT05: Zero activity yields null, never silently 50.0.
- AT08: Warmup lookback periods yield null score.
- AT10: Strictly aggregate buy and sell values before computing ratio; never average individual symbol BB scores.
- AT17: Breadth ratios (count and value) must sum to 1.0 within tolerance $10^{-6}$.
- AT11: Delisted constituent does not contribute after delisting date; newly listed constituent does not contribute before effective date.

## Current architecture
- `backend/app/domain/bb/` currently calculates symbol-level Blackbox under `OHLCV_PROXY` via `ProxyBBCalculator`.
- `backend/app/domain/universe/` provides `UniverseResolver` and `UniverseRegistry` with `POINT_IN_TIME` and `RETROSPECTIVE_FIXED` modes.
- There is currently no multi-symbol whole-market aggregator, no flow breadth calculation, and no market publication gate.

## Target design
- `MarketBBAggregator` acts as a pure, stateless domain calculator taking resolved universe members and daily constituent bar series.
- It converts each constituent's daily bar into active buy/sell values under `OHLCV_PROXY` ($B_i = \frac{1+P_i}{2} V_i, S_i = \frac{1-P_i}{2} V_i$), sums them across eligible reporting constituents to form $\text{MarketBuy}_t$ and $\text{MarketSell}_t$, and rolls them over horizon $H$.
- It computes independent flow breadth across valid symbol scores.
- `evaluate_publication_gate` validates coverage, PIT mode, and candidate status before assigning `MarketPublicationStatus`.

## Milestones
1. **Domain Contracts & Models**: Extend `contracts.py` with Market BB, Breadth, and Publication Status types.
2. **Pure Aggregator Engine**: Implement `market_aggregator.py` with value-weighted aggregation, rolling horizon calculation, independent breadth, and publication gate evaluation.
3. **Pydantic Schemas & Service Integration**: Extend `bb_schema.py`, `bb_service.py`, and `api/bb.py`.
4. **Comprehensive Test Suite**: Author `test_market_bb.py` verifying AT09, AT10, AT11, AT12, AT17, and publication gate rules.
5. **Quality Gates & Independent Review**: Pass focused tests, `verify-v2.ps1`, `run-comprehensive-uat.ps1`, verify sumi.db immutability, issue review seal, and advance `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `BB-MKT-001` (AT09 Aggregate Identity) | `MarketBBAggregator.calculate_market_bb` | `test_at09_aggregate_identity` |
| `BB-MKT-002` (AT10 No Score Averaging) | `MarketBBAggregator` value-weighted sum | `test_at10_no_score_averaging_counterexample` |
| `BB-MKT-003` (AT11 Universe PIT Boundary) | `MarketBBAggregator` with `UniverseResolver` | `test_at11_universe_point_in_time_boundary` |
| `BB-MKT-004` (AT17 Breadth Consistency) | `MarketBBAggregator._calculate_breadth` | `test_at17_breadth_consistency_and_sums` |
| `BB-MKT-005` (Publication Gate & Retrospective Caveat) | `MarketBBAggregator.evaluate_publication_gate` | `test_publication_gate_rules` |

## Verification commands
```powershell
python -m pytest backend/app/tests/test_market_bb.py -v
.\scripts\verify-v2.ps1
.\scripts\run-comprehensive-uat.ps1
Get-FileHash "backend\sumi.db" -Algorithm SHA256
node scripts/verify-dev-program.mjs
```

## Rollback and compatibility
- Revert changes to `backend/app/domain/bb/`, `backend/app/services/bb_service.py`, `backend/app/api/bb.py`, and `backend/app/schemas/bb_schema.py`.
- No database migrations exist, leaving `backend/sumi.db` intact.

## Risks and mitigations
- **Risk**: Survivorship bias in retrospective evaluation.
  - **Mitigation**: Publication gate strictly forces status to `RESEARCH_RETROSPECTIVE` with explicit warning string; `CANONICAL_PUBLISHED` is refused.
- **Risk**: Low constituent coverage distorting market totals.
  - **Mitigation**: Coverage < 50% triggers `UNAVAILABLE_DEGRADED` and suppresses calculations.
- **Risk**: Score averaging misconception.
  - **Mitigation**: Enforce mathematical proof test `test_at10_no_score_averaging_counterexample` in test suite.

## Progress log
- 2026-09-19: ExecPlan created for batch `P7-MKT-02`. Implementation commenced.
- 2026-09-19: Implemented `MarketBBAggregator`, contracts, schemas, service orchestration, API endpoints, and comprehensive tests. All gates verified. Independent review sealed.

## Decision log
- **Decision**: Value conversion for OHLCV_PROXY decomposes trading value into $B_i = \frac{1+P_i}{2} V_i$ and $S_i = \frac{1-P_i}{2} V_i$.
  - **Rationale**: Mathematically guarantees $B_i + S_i = V_i$, $B_i - S_i = P_i V_i$, and $100 \times \frac{B_i}{B_i + S_i} = 50(1 + P_i) = BB_i$. Aggregating $B_i$ and $S_i$ across universe members gives true value-weighted market flow.
- **Decision**: No SQLite tables for Market BB in MVP.
  - **Rationale**: Direct compliance with `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` line 453. Doraemon is the future canonical materializer.

## Completion evidence
- Review Seal: `docs/reviews/P7_MKT_02_REVIEW.md` (ACCEPTED).
- Unit Tests: `backend/app/tests/test_market_bb.py` (8/8 passed).
- Fast Technical Gate: `.\scripts\verify-v2.ps1` (338 backend tests, 210 frontend Vitest tests, ESLint, Vite build - 0 failures).
- Comprehensive Browser UAT: `.\scripts\run-comprehensive-uat.ps1` (31/31 scenarios passed, zero console errors, zero DB mutations).
- Database Immutability: SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` verified unchanged.
- Preserved Staged Deletions: 159 historically staged deletions preserved intact.

