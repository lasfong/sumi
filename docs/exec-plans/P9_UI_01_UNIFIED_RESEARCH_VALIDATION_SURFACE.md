# P9-UI-01 — Unified Research Validation Surface

## Outcome
Deliver an integrated research validation surface uniting the frozen signal registry (72 signals across 11 categories), multi-phase batch backtesting with degradation matrix and CSV export, universe selection, causal active signal explanation inspection, visual rule composition, and Money Flow BB visualization under the audited `OHLCV_PROXY` methodology (`UI-SIG-001/002/003`, `UI-EXP-001`, `FR-CORE-006/010/012`, Master Appendix F). Preserve 100% backward compatibility with all existing Strategy Lab and Replay workflows.

## Context and problem
- **Canonical References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 507–520, `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` sections 7, Master Appendix F (`UI-SIG-001/002/003`, `UI-EXP-001`, F.3, F.4, F.5), `FR-CORE-006`, `FR-CORE-010`, `FR-CORE-012`, and `AGENTS.md`.
- **Problem**:
  1. Sumi's backend delivers 72 registered causal signals across 11 categories (Volume, Pattern, Technical, Regime, Support/Resistance, Structure, VSA, Ichimoku, Divergence, Health, Flow), but the frontend lacks a unified, registry-driven catalog for browsing, inspecting parameters, and checking AST aliases.
  2. The active signal inspector previously only displayed hardcoded volume spike signals; users need causal explanation inspection for all registered signals with score breakdown, quality badges, and structured audit reasons at any observed replay date.
  3. Multi-phase batch evaluation (`PhaseMetricMatrix`, degradation, consistency scoring, and CSV export delivered in P4-BATCH-01 / P4-MET-02) and managed universes (VN30, HOSE50 delivered in P7-UNI-01) currently lack a dedicated UI surface in Strategy Lab.
  4. Money Flow BB lines and events must be visualized with explicit `OHLCV_PROXY` methodology and data quality badges, without misleading traders into believing proxy flow represents actual exchange order flow.
  5. The DSL rule composition must assist users in selecting valid AST aliases and parameter ranges rather than requiring arbitrary code typing.

## In scope
1. **Signal Catalog (`UI-SIG-001`, `UI-SIG-002`, `UI-SIG-003`)**:
   - Browse all 72 registered signals by category: Pattern, Technical Signal, Volume, Regime, VSA, Structure, Ichimoku, Divergence, Health, Flow, Support/Resistance.
   - Signal cards show: Vietnamese label, short meaning/description, parameters + defaults, version, status badge (`ACTIVE`, `EXPERIMENTAL`), AST alias.
   - Default view hides advanced/research parameters (`UI-SIG-003`) with a toggle ("Xem tham số nâng cao" / "Ẩn tham số nâng cao").
   - Instant search/filter by keyword, name, or Vietnamese label.
2. **Active Explanation / Date Inspector (`UI-EXP-001`)**:
   - Allows selecting signals from the catalog for live causal inspection during replay.
   - Displays active state, score, quality (`VALID`, `INSUFFICIENT_HISTORY`, `INVALID_VOLUME`, `ZERO_BASELINE`), and structured audit reasons.
   - Supports 4-family Technical Health score inspection with missing-BB neutrality status.
   - Supports BB flow events with method badge (`OHLCV_PROXY`) and quality badge (`HIGH`, `MEDIUM`).
3. **Strategy Rule Builder / Composer (`FR-CORE-006`, F.4)**:
   - Visual rule constructor from registered signals and operators (`>`, `<`, `==`, `and`, `or`, `not`).
   - Validates parameter bounds and types before execution.
4. **Multi-Phase Batch Backtest & Phase Metrics Table / Export (`FR-CORE-010`, `FR-CORE-012`, F.5)**:
   - Phase editor: define multi-phase date ranges (e.g. In-Sample vs Out-of-Sample, Bull / Bear phases).
   - Universe selector: integrate `/api/universe/list` (e.g. VN30, HOSE50) or custom ticker lists.
   - Invokes `/api/backtest/batch/run`.
   - Displays 2D `PhaseMetricMatrix`: Net Return %, CAGR, Max Drawdown, Win Rate, Profit Factor, Sharpe, Calmar, Volatility, Consistency Score.
   - Cross-Phase Degradation table: return delta, win rate delta, degradation ratio, `is_degraded` badge.
   - Correct negative sign formatting (`-X.XX%`), `N/A` for missing values, zero future rows.
   - CSV export functionality (`FR-CORE-012`).
   - Warnings & Data Quality Panel: surfaces assumptions (Proxy flow methodology `OHLCV_PROXY`, warmup requirements, conservative settlement `T+2.5`/`T+2`).
5. **Technical Flow BB Chart / Visualizer (F.3)**:
   - Expose BB horizons: `BB03`, `BB05`, `BB20`, `BB50`, `BB200` with optional `BB10` toggle.
   - Prominent `OHLCV_PROXY` methodology badge and audit disclaimer tooltip.
   - Data quality badges (`HIGH`, `MEDIUM`, `LOW`).
6. **Strategy Lab & Replay Workspace Integration**:
   - Integrate into `StrategyLabPage.tsx` via clean tabbed workspace: "⚔️ Strategy Battle", "⚡ Parameter Sweep", "📊 Multi-Phase Matrix & Universes", "📚 Signal Catalog & Builder", "📜 History".
   - Integrate Signal Catalog modal and enhanced explanation inspector in `ReplayWorkspace.tsx`.

## Out of scope
- Mutating backend database schemas or `backend/sumi.db`.
- Creating a completely new routing structure or discarding existing user-owned Strategy Lab features.
- Allowing arbitrary Python code execution on the frontend or widening the backend AST whitelist.

## Invariants
- **No Future Leak**: Signal inspection and backtest phase metrics evaluate strictly through observed indices.
- **BB Methodology Transparency**: `OHLCV_PROXY` is always prominently identified as an estimation from price/volume, never labeled as actual direct exchange capital flow.
- **Missing-BB Neutrality**: Absence of BB data is displayed as neutral, not negative evidence.
- **Zero Database Mutation**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` and 159 staged deletions remain intact.
- **Backward Compatibility**: All existing 31 UAT test cases and 210 frontend unit tests must continue to pass cleanly.

## Current architecture
- `frontend/src/pages/StrategyLabPage.tsx`: monolithic battle, parameter sweep, and history page.
- `frontend/src/components/signals/SignalInspector.tsx`: single-signal volume spike inspector with rigorous causal debounce.
- `frontend/src/api/`: `backtestApi.ts` (single run backtest only), `signalsApi.ts` (catalog & replay calculate).
- Backend: `/api/signals/registry`, `/api/signals/replay/{sessionId}/calculate`, `/api/backtest/batch/run`, `/api/universe/list`, `/api/bb/*`.

## Target design
1. **API Clients**:
   - `frontend/src/api/backtestApi.ts`: add `runBatchBacktest` with full types for `BatchBacktestRequest`, `BatchBacktestResponse`, `PhaseMetricMatrixResponse`, `CrossPhaseDegradationResponse`.
   - `frontend/src/api/universeApi.ts`: add `listUniverses`, `resolveUniverse`.
   - `frontend/src/api/bbApi.ts`: add `getBBHorizons`, `getSymbolBB`.
2. **Components**:
   - `frontend/src/components/signals/SignalCatalog.tsx`: Category tabs, search input, signal cards with Vietnamese labels, parameter schemas, defaults, versions, status badges, AST aliases, and advanced parameter toggle.
   - `frontend/src/components/signals/SignalCatalogModal.tsx`: Modal wrapper for launching the catalog from Replay or Strategy Lab.
   - `frontend/src/components/signals/SignalExplanationInspector.tsx`: Generic active signal explanation inspector supporting any registered signal, multi-signal selection, 4-family health breakdown, and BB flow events.
   - `frontend/src/components/strategy/MultiPhaseBatchPanel.tsx`: Phase range editor, universe picker, batch run executor, PhaseMetricMatrix table, CrossPhaseDegradation table, CSV download, and data quality warnings panel.
   - `frontend/src/components/strategy/StrategyRuleBuilder.tsx`: Visual rule composition with registered signals and operators.
   - `frontend/src/components/chart/TechnicalFlowBBViewer.tsx`: BB horizons visualization with `OHLCV_PROXY` methodology badge and quality indicator.
3. **Page Integration**:
   - `StrategyLabPage.tsx`: Tabbed view seamlessly hosting Battle, Parameter Sweep, Multi-Phase Batch & Universe, Signal Catalog & Rule Builder, and History.
   - `ReplayWorkspace.tsx`: Add button to open Signal Catalog modal and embed generic explanation inspector.

## Milestones
1. **Milestone 1**: Implement frontend API clients (`universeApi.ts`, `bbApi.ts`, and batch types in `backtestApi.ts`).
2. **Milestone 2**: Implement `SignalCatalog.tsx` and `SignalCatalogModal.tsx` (`UI-SIG-001/002/003`).
3. **Milestone 3**: Implement `SignalExplanationInspector.tsx` (`UI-EXP-001`) with multi-signal support, Health 4-family breakdown, and BB flow events.
4. **Milestone 4**: Implement `MultiPhaseBatchPanel.tsx` (`FR-CORE-010/012`, F.5) with phase editor, universe picker, degradation matrix table, CSV export, and data quality warnings panel.
5. **Milestone 5**: Implement `StrategyRuleBuilder.tsx` (`FR-CORE-006`, F.4) and `TechnicalFlowBBViewer.tsx` (F.3).
6. **Milestone 6**: Integrate components into `StrategyLabPage.tsx` and `ReplayWorkspace.tsx`.
7. **Milestone 7**: Write comprehensive unit/integration tests for new components, verify fast technical gate (`verify-v2.ps1`), browser E2E UAT (`run-comprehensive-uat.ps1`), emit independent review seal (`docs/reviews/P9_UI_01_REVIEW.md`), and update `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test evidence |
|---|---|---|
| `UI-SIG-001` | `frontend/src/components/signals/SignalCatalog.tsx` | Category browsing across all 11 categories in unit & UAT tests |
| `UI-SIG-002` | `frontend/src/components/signals/SignalCatalog.tsx` | Signal cards with Vietnamese labels, descriptions, parameters, versions, status badges |
| `UI-SIG-003` | `frontend/src/components/signals/SignalCatalog.tsx` | Toggle hiding/revealing advanced research parameters |
| `UI-EXP-001` | `frontend/src/components/signals/SignalExplanationInspector.tsx` | Active signal explanation, score, reasons, and Health/BB breakdown |
| `FR-CORE-006` | `frontend/src/components/strategy/StrategyRuleBuilder.tsx` | Safe visual rule composition from registered AST aliases |
| `FR-CORE-010` | `frontend/src/components/strategy/MultiPhaseBatchPanel.tsx` | Multi-phase batch backtest execution and 2D PhaseMetricMatrix |
| `FR-CORE-012` | `frontend/src/components/strategy/MultiPhaseBatchPanel.tsx` | Cross-phase degradation table and CSV export |
| Master F.3 / F.5 | Methodology & warning panels | Prominent `OHLCV_PROXY` badge, quality badge, settlement & warmup disclaimers |

## Rollback plan
If any gate or verification fails:
1. Revert changes to `frontend/src/pages/StrategyLabPage.tsx`, `frontend/src/components/replay/ReplayWorkspace.tsx`, and `frontend/src/api/backtestApi.ts`.
2. Remove newly added components in `frontend/src/components/signals/` and `frontend/src/components/strategy/`.
3. Verify git status and test suite to ensure clean return to baseline.

## Verification evidence
- **Frontend Linter**: `npm run lint` -> 0 errors, 0 warnings (Exit Code 0).
- **Frontend Unit Tests**: `npm test` -> 37 test files, 226 tests passed (100% green).
- **Frontend Production Build**: `npm run build` -> clean build in 589ms (Exit Code 0).
- **Fast Technical Gate**: `.\scripts\verify-v2.ps1` -> all backend and frontend gates passed (Exit Code 0).
- **Comprehensive Browser E2E UAT**: `.\scripts\run-comprehensive-uat.ps1` -> 31/31 passed (100%), 0 console errors, 0 DB mutations (Exit Code 0).
- **Data Invariant**: `backend/sumi.db` SHA-256 baseline `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` strictly preserved.
- **Git Invariant**: 159 historically staged deletions intact.
- **Independent Review Seal**: Accepted in `docs/reviews/P9_UI_01_REVIEW.md`.
