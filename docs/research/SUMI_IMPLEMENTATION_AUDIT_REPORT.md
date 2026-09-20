# SUMI Implementation Audit Report

**Audit date:** 2026-09-12  
**Scope:** `READ -> AUDIT -> MAP -> PLAN -> STOP`; no production feature implementation  
**Repository:** `E:\Workspace\sumi` current working tree  
**Production database:** `backend/sumi.db`, opened with SQLite `mode=ro` and `PRAGMA query_only=ON`  
**Database SHA-256 after audit:** `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`

## Audit basis and confidence

The following were read before reaching architecture conclusions:

- `AGENTS.md`, `PLANS.md`, `README.md`, `docs/INDEX.md`.
- `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`.
- `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`.
- `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`.
- `docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md`.
- The complete `docs/research/SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md`.
- `MASTER_SPEC.md` and every supporting document in `docs/research/SUMI_Money_Flow_Blackbox_V1_Handoff/`.
- Backend domain, services, models, migrations, APIs, tests, provider adapters, frontend API/domain/composition code, verification scripts, production schema/data, and retained UAT result JSON.

Only the root `AGENTS.md` exists in the current working tree. A previously tracked `docs/AGENTS.md` is staged as deleted, so no live nested instruction file applies. The working tree was already heavily dirty before this audit (modified backend/frontend files, many staged documentation deletions, and untracked V3/research files). Those changes were treated as user-owned. Findings describe the **current checkout**, not a clean committed release baseline.

No repository instruction conflicts with the requested audit workflow. There is, however, a product-scope distinction that future work must preserve: current V3 documentation intentionally removes cash/T+2 barriers from **manual TA practice**, while the Master Spec requires Vietnam settlement and cash correctness in **automated backtests**. These are separate execution profiles, not one shared permissive lifecycle.

> **Cross-repository follow-up (2026-09-12):** The Sumi-local findings below remain unchanged. The subsequent [Doraemon Market Data Audit](DORAEMON_MARKET_DATA_AUDIT.md) establishes that the production backend adds real KBS daily OHLCV, sparse actual trading value and recent foreign volume, plus current-session trade prints and quotes. It still does not establish historical trustworthy aggressor side, synchronized historical quotes, or a complete point-in-time market universe. The combined recommendation therefore remains `OHLCV_PROXY` for the Sumi MVP, with a separate current-session `TICK_TEST_ESTIMATE_RESEARCH_ONLY` spike possible and KBS side labels retained as `UNKNOWN` until validated.

## 1. Executive Summary

### Readiness

Sumi has a real and useful V3 foundation: a backend-authoritative 16-indicator registry, replay APIs that normally slice candles through `current_index`, a safe AST/declarative rule evaluator, manual practice/order persistence, single- and multi-symbol strategy runs, analytics, a provider boundary, React indicator/drawing state, and extensive automated tests.

It is **not implementation-ready as the requested Price/Volume/Money-Flow quantitative platform without staged extension and backtest correction**. The dedicated Signal Engine, Signal Registry, explainable signal result contract, Money Flow module, point-in-time universe, multi-phase runner, exact nine-metric benchmark output, and Vietnam calendar/rules provider are absent.

### Highest-risk findings

1. **Automated backtest look-ahead / execution-timing defect:** `BacktestService` evaluates rules using `Close[T]` and immediately creates `MARKET_AT_CLOSE` fills at that same close. `ExecutionAssumptions` explicitly reports this behavior. This violates `BT-TIME-001` and `TEST-BT-001`.
2. **Backtest execution is coupled to the permissive manual-practice lifecycle:** it reuses `TradeLifecycleService`, whose current product behavior deliberately does not enforce cash or T+2 for practice. The backtest adds only an `i - buy_index >= 2` guard and has no settlement-lot/calendar model.
3. **Configured backtest risk rules are inert:** `risk_management.stop_loss_pct` and `take_profit_pct` are parsed but never applied by `BacktestService`; trailing stops do not exist in this path.
4. **Ichimoku replay payload risk:** `IndicatorEngine` deliberately unions pandas-ta result indexes, including projected future cloud dates, and the replay indicator API serializes the full result. Tests assert future index extension. The values may be causal projections, but future-dated indicator records violate the current replay acceptance wording unless an explicit projected-data contract is adopted.
5. **Money Flow Data Audit Gate fails for TRUE flow:** the database contains only OHLCV and source metadata. There are no trade ticks, aggressor side, historical bid/ask, matched value, put-through fields, investor-class flows, or auditable raw payloads.
6. **Whole-market BB is blocked:** every one of 3,734 `symbols.exchange` values is `NULL`, and there are no listing, delisting, suspension, security-type history, or point-in-time membership tables.
7. **Provider documentation and implementation diverge:** the SSI and vnstock adapters are deterministic synthetic weekday candle generators. They do not call SSI or vnstock. ADR-002 and handoff language describing real provider coverage is therefore not verified by code.
8. **Historical stream ambiguity:** backtest/scanner candle queries omit `timeframe` and `adjustment_type`, so mixed streams can enter calculations where such rows exist. The database currently contains 2,403,639 `1D/unadjusted` rows plus three `D/split` rows.
9. **The retained UAT report overstates several scenarios:** for example, its auto SL/TP case passes while evidence shows zero closed trades, and its previous-step case reports bar 1 before and after. Treat the report as UI smoke evidence, not proof of new Master Spec correctness.

### Feasibility and blockers

- Price/Volume Signal Engine: **feasible** by extending the current `IndicatorEngine` and safe DSL.
- Correct causal batch backtest: **feasible**, but requires a distinct backtest execution domain and versioned market-rule assumptions.
- Symbol-level Money Flow now: only **`OHLCV_PROXY`** is supported by available data, using `ESTIMATED_TP_X_VOLUME`; even this requires explicit adjustment/corporate-action policy.
- `EXECUTED_ORDER_FLOW`: **blocked by missing data and semantics**.
- `CLASSIFIED_ORDER_FLOW`: **blocked by missing trade and quote history**.
- Historical Market BB / Breadth: **blocked by missing point-in-time universe/lifecycle metadata**.

### Recommended next step

Approve one bounded implementation slice for the **Signal Foundation + Volume Spike vertical capability**, while separately opening data-source and Vietnam-market-rule discovery spikes. Do not connect new signals to the current automated backtest until the same-close execution defect is corrected.

## 2. Verified Repository Architecture Map

### Reported baseline versus verified checkout

| Item | Classification | Evidence / actual state |
| --- | --- | --- |
| Python 3.13 | VERIFIED | Local venv reports Python 3.13.13. |
| FastAPI / Pydantic v2 / SQLAlchemy / Pandas / NumPy / pandas-ta / PyYAML / SQLite / Alembic | VERIFIED | `backend/requirements.txt`; installed versions include FastAPI 0.138.1, Pydantic 2.13.4, SQLAlchemy 2.0.51, Pandas 2.3.3, NumPy 2.2.6. Requirements are mostly unpinned. |
| React 19 / TypeScript / Vite / TanStack Query / Lightweight Charts | VERIFIED | `frontend/package.json`: React 19.2.6, Vite 8.0.16, TanStack Query 5.101.0, Lightweight Charts 5.2.0. |
| `IndicatorEngine` | VERIFIED | `backend/app/domain/engine/indicator_engine.py`; registry-backed backend authority. |
| `StrategyRuleEvaluator` | VERIFIED | `backend/app/domain/strategy/strategy_rule_evaluator.py` and `rule_evaluator.py`. |
| `ReplayService`, `TradeLifecycleService`, `PracticeWorkflowService`, `StrategyLabService` | VERIFIED | Concrete service modules exist. |
| `MarketDataProviderAdapter` | VERIFIED | Abstract boundary and DTOs exist in `backend/app/services/data_providers/base_provider.py`. |
| Real SSI FastConnect adapter | DIFFERENT_FROM_REPORTED | `ssi_provider.py` generates deterministic synthetic candles and performs no HTTP/auth exchange. |
| Real vnstock adapter | DIFFERENT_FROM_REPORTED | `vnstock_provider.py` also generates deterministic synthetic candles and does not invoke vnstock. |
| CafeF import | VERIFIED_WITH_LIMITS | Local CSV/ZIP OHLCV parser and transactional preview/accept workflow exist; production `import_runs` has zero rows and original raw files are absent. |
| Dedicated Signal Engine / Signal Registry | NOT_FOUND | No signal domain package, registry, reason contract, or signal API exists. |
| MoneyFlowBlackbox | NOT_FOUND | No money-flow domain/API/persistence implementation exists. MFI is only a technical oscillator. |
| Multi-phase backtest | NOT_FOUND | Request supports one `start_date/end_date`; result slices are post-hoc symbol/year/regime groupings, not independent phase execution. |

### Backend responsibility map

| Concern | Concrete path / symbol | Verified behavior |
| --- | --- | --- |
| App composition | `backend/app/main.py` | Mounts health, import, replay, decision, analytics, symbol, journal, indicators, WebSocket, backtest, scanner, strategy-lab, and sync routers. |
| Candle model | `backend/app/models/candle.py:Candle` | Unique `(symbol,timeframe,timestamp,adjustment_type)` OHLCV rows plus source. |
| Symbol model | `backend/app/models/symbol.py:Symbol` | Current-state identifier/exchange/company/sector/asset/is_active only; no lifecycle history. |
| Indicator registry/calculation | `backend/app/domain/engine/indicator_engine.py:INDICATOR_REGISTRY`, `IndicatorEngine` | 16 definitions: SMA, EMA, MACD, RSI, BBands, ATR, ADX, Ichimoku, Stochastic, Volume SMA, PSAR, SuperTrend, CCI, MFI, Keltner, Relative Strength. Most delegate to pandas-ta; CCI and RS are custom. |
| Strategy indicator mapping | `backend/app/domain/engine/strategy_indicator_adapter.py` | Computes each configured indicator independently, maps dataframe columns to scalar arrays, adds raw OHLCV namespaces. |
| Safe expression/DSL | `backend/app/domain/strategy/rule_evaluator.py` | AST whitelist plus declarative `all/any/not/gt/gte/lt/lte/eq/cross_up/cross_down/between/rising/falling`; rejects calls, attributes, imports, arithmetic, and unknown identifiers. |
| Strategy schema | `backend/app/domain/strategy/strategy_schema.py` | Indicator list, entry/exit rule dictionaries, fixed/percent sizing, optional stop/take-profit percentages. Cross-field and strict-extra validation are limited. |
| Scanner | `backend/app/services/scanner_service.py` | Per-symbol candle query, indicator precompute, scalar rule loop, chronological unranked entry signals. |
| Backtest orchestration | `backend/app/services/backtest_service.py` | Sequential independent symbol runs; creates persisted replay sessions/trades; computes indicators once per current single date range; evaluates/fills at close. |
| Manual execution | `backend/app/services/trade_lifecycle_service.py` | Decisions, pending next-open/limit orders, positions, executions, fees/tax, SL/TP automation during replay advancement; practice cash is intentionally non-blocking. |
| Alternative simulation kernel | `backend/app/domain/engine/broker.py`, `models.py`, `events.py` | Separate event-driven broker with slippage, cash check, index-based T+2; not imported by production services. This is duplicate/orphan execution logic. |
| Analytics | `backend/app/services/analytics_service.py` | Equity curve, drawdown, Sharpe/Sortino/SQN, typed status metadata, grouping and trade distributions; not the exact nine-metric Master output. |
| Data import/sync | `cafef_importer.py`, `import_classifier.py`, `import_workflow_service.py`, `sync_workflow_service.py` | Transactional OHLCV preview/accept/rollback and weekly derivation. Provider fetches are synthetic at present. |
| Replay no-future boundary | `backend/app/services/replay_service.py:get_candles` | Primary session candles are limited to `current_index + 1`; MTF rows are bounded by current timestamp. |
| Indicator APIs | `backend/app/api/indicators.py`, `backend/app/api/replay.py` | Registry plus global and replay-scoped calculations. Global endpoint explicitly warns it is not replay safe. |

### API surface relevant to the new specification

- `/api/indicators/registry`, `/api/indicators/{symbol}`.
- `/api/replay/sessions`, `/{id}`, `/{id}/candles`, `/{id}/next`, `/{id}/previous`, `/{id}/indicators`, drawing/practice/journal/order/trade endpoints.
- `/api/backtest/run`, `/api/backtest/strategies`, `/api/backtest/cleanup-sessions`.
- `/api/scanner/run`, scanner history, scanner-to-replay creation.
- `/api/strategy-lab/validate`, `/parameters`, `/sweep`, run history.
- `/api/import/*` and `/api/sync/*`.
- No `/api/signals/*`, Money Flow, phase-batch, exact benchmark-table, or trade-ledger export contract.

### Frontend map

| Concern | Concrete paths | Verified behavior |
| --- | --- | --- |
| Route composition | `frontend/src/App.tsx` | Dashboard, Replay, Backtest, Scanner, Analytics, Journal, Import, Strategy Lab. |
| Replay composition | `pages/ReplayPage.tsx`, `components/replay/ReplayWorkspace.tsx`, `ReplayWorkspaceController.tsx` | Controller coordinates session, replay, indicators, drawing, practice, journal and navigation. |
| Chart/provider boundary | `components/chart/CandleChart.tsx`, `SeriesManager.ts`, `PaneManager.ts`, `IndicatorRenderRegistry.ts`, `SumiDrawingAdapter.ts` | Lightweight Charts calls are substantially isolated behind chart helpers/adapters. |
| Indicator product state | `features/indicators/indicatorDomain.ts`, `IndicatorRepository.ts`, `IndicatorRequestCoordinator.ts` | Versioned instances preserve type, params, pane, visibility, style and order; session persistence is local/frontend repository based. |
| Strategy UI | `pages/BacktestPage.tsx`, `pages/StrategyLabPage.tsx`, `api/backtestApi.ts` | Single date range, one/many symbols, preset/YAML strategy use, battle/sweep and analytics rendering. No signal catalog or multi-phase editor. |

### Database tables

Schema head is `a2d639ecd5d5`. Tables are: `alembic_version`, `candles`, `symbols`, `replay_sessions`, `decisions`, `orders`, `executions`, `positions`, `trades`, `drawing_states`, `journal_entries`, `event_logs`, `scanner_runs`, `strategy_lab_runs`, `import_runs`, `import_run_items`, `import_run_mutations`, `sync_runs`, `sync_run_items`, `sync_run_mutations`, and `weekly_candle_provenance`.

No table stores signal definitions/results, BacktestRun/phase metadata, settlement lots, market calendars/rule versions, corporate actions, trade/quote ticks, investor flow, BB output, or point-in-time universes.

## 3. Current Capability Assessment

| Subsystem | Status | Assessment |
| --- | --- | --- |
| Indicator layer | EXISTS_NEEDS_EXTENSION | Strong reusable authority and registry. Needs explicit output contract, shared dependency computation, causal/index boundary rules, adjustment policy, and signal primitives. Keep it; do not rewrite wholesale. |
| Indicator warm-up/NaN | EXISTS_NEEDS_EXTENSION | pandas-ta produces NaNs; API converts NaN to null and frontend has warming states. Warm-up is inferred from first finite outputs, not registry metadata. |
| Indicator adjusted/unadjusted policy | EXISTS_NEEDS_REFACTOR | Replay filters the session adjustment type, but global indicator, scanner and backtest paths do not consistently filter it; analysis/execution streams are not separated. |
| Signal Engine | MISSING | No dedicated atomic/composite signal implementation, explanation result, availability timestamp, or versioned signal registry. |
| Pattern / technical trigger signals | MISSING | DSL can express some crosses but no registered, reusable, explainable signal outputs exist. |
| Volume Spike / VSA / support-resistance / candle structure / Technical Health | MISSING | No matching production code found. |
| Trend/regime | EXISTS_NEEDS_REFACTOR | `RegimeClassifier` is a compact benchmark classifier, not the Master Spec signal catalog and lacks registry/evidence contract. |
| Ichimoku indicator | EXISTS_NEEDS_EXTENSION | Numeric indicator and rendering exist; bullish/bearish score/reasons do not. Future-projection payload boundary requires resolution. |
| Divergence | MISSING | No confirmed pivot model or four-way divergence. |
| Strategy DSL | EXISTS_AND_COMPATIBLE | Security boundary is good and should remain. It is scalar-per-bar and identifier-based; future enum/explanation namespaces require a typed adapter. |
| Backtest orchestration | EXISTS_NEEDS_REFACTOR | Useful scaffolding and manifests exist, but execution semantics are non-causal at fill time and mixed with practice persistence. |
| Multi-ticker independent capital | EXISTS_NEEDS_EXTENSION | Symbols run sequentially with separate sessions/cash. This matches independent allocation semantics, but lacks batching and exact comparison output. |
| Multi-phase | MISSING | No phase request or compute-once/slice-many path. |
| Vietnam market rules | EXISTS_NEEDS_REFACTOR | Static fee/tax constants, static price bands, 100-share lot, and bar-index T+2 fragments exist. No date/exchange/status rule provider or calendar. |
| Trade ledger / metrics | EXISTS_NEEDS_EXTENSION | Persisted executions/trades and strong analytics exist, but required timestamps, bars-held and exact nine metrics/conventions are incomplete. |
| Money Flow / Blackbox | MISSING | No domain module, schema, tests, API or UI. |
| Market data provider boundary | EXISTS_AND_COMPATIBLE | Abstract boundary and sync workflow can be extended. Concrete named providers are synthetic and need honest demo labeling or real spikes. |
| Point-in-time universe | MISSING | Current symbol table is insufficient. |
| Performance architecture | EXISTS_NEEDS_REFACTOR | Per-symbol queries, per-indicator dataframe copies, repeated replay computations and scalar execution loops; no feature cache or phase reuse. |
| Frontend signal/backtest configuration | MISSING | No Signal Catalog, explanation UI, BB UI, multi-phase editor, data-quality/method boundary panel, or exact results table. |

## 4. Master Spec Requirement Gap Matrix

Statuses refer to the five audit classifications required by the brief.

### System and non-functional requirements

| Requirement ID | Spec section | Current implementation | Status | Gap / recommended action | Affected modules | Dependencies | Risk | Required tests | Suggested phase |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FR-CORE-001 | 2A | OHLCV indicators exist; no three-axis feature model | MISSING | Create separate Price, Volume and Money Flow namespaces; never blend primitives | new `domain/signals`, future `domain/money_flow` | Signal contract | High semantic | namespace/contract tests | 1/3 |
| FR-CORE-002 | 2A | Backend `IndicatorEngine` is authoritative | EXISTS_AND_COMPATIBLE | Retain; add versions/data-quality/causal metadata | `indicator_engine.py` | none | Low | determinism/golden | 1 |
| FR-CORE-003 | 2A | No Signal Engine | MISSING | Add modular Signal Engine; prohibit trade execution imports | new signal package | shared features | High | boundary tests | 1-2 |
| FR-CORE-004 | 2A | Safe AST/declarative evaluator exists | EXISTS_AND_COMPATIBLE | Add signal namespace mapping without widening AST | strategy evaluators | Signal Registry | Medium | security/identifier tests | 1-2 |
| FR-CORE-005 | 2A | Backtest consumes indicator arrays, but not precomputed signals | EXISTS_NEEDS_EXTENSION | Compile/precompute signals outside execution state machine | backtest + signals | corrected backtest kernel | High | instrumentation test | 4 |
| FR-CORE-006 | 2A | Indicator params configurable; signals absent | EXISTS_NEEDS_EXTENSION | Registry-driven signal parameters and UI | signals/API/frontend | registry | Medium | schema/UI validation | 1/5 |
| FR-CORE-007 | 2A | No deterministic signal evidence contract | MISSING | Implement reasons, score, values, version/hash/quality | signals schemas | registry | Medium | explanation golden tests | 1-2 |
| FR-CORE-008 | 2A | Multi-symbol only; single date range | EXISTS_NEEDS_EXTENSION | Add phase list and batch run model | backtest API/service/domain | phase policy | High | multi-symbol/multi-phase | 4 |
| FR-CORE-009 | 2A | Each symbol gets a separate session/capital | EXISTS_AND_COMPATIBLE | Preserve explicitly; do not convert to portfolio cash | backtest service | none | Low | independent-capital test | 4 |
| FR-CORE-010 | 2A | Rich analytics but not exact nine metrics | EXISTS_NEEDS_EXTENSION | Add authoritative benchmark metrics/result schema | metrics/reporting/UI | corrected ledger | High | exact formulas/N/A | 4 |
| FR-CORE-011 | 2A | Replay candle boundary is good; backtest same-close fill and Ichimoku future index are unresolved | EXISTS_NEEDS_REFACTOR | Correct timing and add future-invariance suite | replay/indicator/backtest | execution profile | Critical | causal/truncation | 0/1/4 |
| FR-CORE-012 | 2A | No BB | MISSING | Implement only after Data Audit Gate, with method/quality metadata | money flow | source evidence | Critical | AT01-AT20 | 3 |
| FR-CORE-013 | 2A | Workstreams can be separated | EXISTS_AND_COMPATIBLE | Keep Price/Volume and backtest independent of BB | plans/contracts | none | Low | dependency-boundary test | all |
| FR-CORE-014 | 2A | Run manifest hashes strategy/data/assumptions | EXISTS_NEEDS_EXTENSION | Persist execution/market/signal/BB versions, phases and snapshot identity | schemas/models/service | migrations | Medium | rerun reproducibility | 4 |
| NFR-PERF-001 | 2B | Indicators once per current symbol/range; no phases | EXISTS_NEEDS_EXTENSION | Feature matrix once, phases sliced afterward | batch runner/cache | signal graph | High | call-count instrumentation | 4 |
| NFR-PERF-002 | 2B | No formal 30-50 x 3 benchmark | UNKNOWN | Benchmark after architecture exists; record reference machine/data | perf harness | phases/cache | Medium | performance benchmark | 5 |
| NFR-DET-001 | 2B | Deterministic calculations and input hash mostly present | EXISTS_NEEDS_EXTENSION | Add versioned signal/market/flow specs and golden fixtures | all domain modules | version policy | Medium | golden/repro | 1-5 |
| NFR-OBS-001 | 2B | Mixed strings/HTTP errors; no structured module/requirement context | EXISTS_NEEDS_EXTENSION | Standardize domain error context and warnings | services/schemas | contracts | Medium | error payload tests | 1/4 |
| NFR-COMP-001 | 2B | Python 3.13/Windows gate passes | EXISTS_AND_COMPATIBLE | Preserve | manifests/scripts | pinned dependencies recommended | Low | verify gate | all |
| NFR-MAINT-001 | 2B | No signal monolith because no Signal Engine | MISSING | Implement category modules and dependency graph | signals | registry | Medium | import/boundary review | 1-2 |
| NFR-UX-001 | 2B | Indicator forms are bounded; signal forms absent | EXISTS_NEEDS_EXTENSION | Keep 2-5 user-facing signal params; hide research controls | registry/frontend | signal schema | Low | UI schema test | 5 |
| NFR-DQ-001 | 2B | Some typed analytics statuses; no feature/signal DQ propagation | EXISTS_NEEDS_EXTENSION | Add explicit GOOD/DEGRADED/INSUFFICIENT/MISSING | domain schemas | data audit | High | missing/null tests | 1/3 |

### Signal contracts

| Requirement ID | Spec section | Current implementation | Status | Gap / recommended action | Affected modules | Dependencies | Risk | Required tests | Suggested phase |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SIG-PAT-001, SIG-PAT-002 | D.1 | None | MISSING | Bullish/bearish engulfing with quality/reasons | `signals/patterns.py` | candle features | Medium | boundary/NaN | 2 |
| SIG-PAT-003, SIG-PAT-004 | D.1 | None | MISSING | Hammer/pinbar and shooting-star symmetric rules | patterns | candle features | Medium | zero-range/context | 2 |
| SIG-PAT-005, SIG-PAT-006 | D.1 | None | MISSING | Morning/evening star, no mandatory gap | patterns | candle features | Medium | 3-bar golden | 2 |
| SIG-PAT-007 | D.1 | None | MISSING | Piercing/dark-cloud midpoint rules | patterns | candle features | Medium | midpoint edges | 2 |
| SIG-PAT-008 | D.1 | None | MISSING | Tweezer ATR tolerance | patterns | causal ATR | Medium | tolerance edges | 2 |
| SIG-PAT-009 | D.1 | None | MISSING | Inside-bar breakout/breakdown | patterns | causal ATR | Medium | reference exclusion | 2 |
| SIG-PAT-010 | D.1 | None | MISSING | Selected-pattern aggregate with child reasons | patterns/registry | child signals | Low | selected children | 2 |
| SIG-TECH-001, SIG-TECH-002, SIG-TECH-003, SIG-TECH-004 | D.2 | DSL can express crosses; no registered outputs | EXISTS_NEEDS_EXTENSION | Precompute MACD/RSI/EMA cross events and persistent state separately | `signals/technical.py` | indicator outputs | Medium | equality/cross tests | 2 |
| SIG-TECH-005 | D.2 | None | MISSING | Break only confirmed swing | technical/pivots | confirmed pivot | High | availability delay | 2 |
| SIG-TECH-006 | D.2 | DSL `all/any`; no at-least-N composite/reasons | MISSING | Registered threshold-count composite | technical/registry | children | Medium | count/reasons | 2 |
| SIG-VOL-001 | D.3 | Volume SMA includes current bar; no relative volume | MISSING | Prior-N baseline excluding current | `signals/candle_features.py` | raw volume | High causal | exact rolling/warmup | 1 |
| SIG-VOL-002 | D.3 | None | MISSING | First vertical signal: Volume Spike + optional filters | `signals/volume.py` | SIG-VOL-001 | Medium | thresholds/reasons/future invariance | 1 |
| SIG-VOL-003 | D.3 | None | MISSING | Technical climax labels | volume | ATR/candle features | Medium | bounds/labels | 2 |
| SIG-REG-001, SIG-REG-002 | D.4 | Separate `RegimeClassifier` does not match contract | EXISTS_NEEDS_REFACTOR | Registered EMA alignment/slope states | `signals/regimes.py` | EMA | Medium | slope/warmup | 2 |
| SIG-REG-003 | D.4 | No ADX/BBW sideways signal | MISSING | Versioned simple preset | regimes | ADX/BBands | Medium | preset golden | 2 |
| SIG-REG-004, SIG-REG-005 | D.4 | None | MISSING | Contextual correction/recovery, distinct states | regimes | prior regimes/SR | High | transition/causal | 2 |
| SIG-REG-006 | D.4 | None | MISSING | New high/low excluding current reference bar | regimes | rolling features | Critical causal | truncation/reference tests | 1-2 |
| SIG-VSA-001, SIG-VSA-002, SIG-VSA-003, SIG-VSA-004, SIG-VSA-005, SIG-VSA-006 | D.5 | None | MISSING | Modular technical-inference VSA subset and composites | `signals/vsa.py` | candle/volume/SR | High semantic | zero-range/causal/reasons | 2 |
| SIG-SR-001 | D.6 | None | MISSING | ATR-normalized prior/confirmed support/resistance | `signals/support_resistance.py` | ATR/pivots | High causal | current exclusion/pivot delay | 1-2 |
| SIG-STR-001 | D.7 | None | MISSING | Transparent candle score and good/bad flags | `signals/structure.py` | candle features | Medium | zero-range/bounds | 2 |
| SIG-HEALTH-001 | D.8 | None | MISSING | Versioned non-overlapping component preset and missing-component semantics | `signals/health.py` | trend/momentum/volume/optional BB | High semantic | missing BB/no double count | 2/3 |
| SIG-ICHI-001 | D.9 | Ichimoku numeric indicator exists | EXISTS_NEEDS_EXTENSION | Add causal score/state/reasons; separate raw vs displayed spans | `signals/ichimoku_signals.py`, indicator adapter | Ichimoku contract decision | Critical causal | displacement/truncation | 2 |
| SIG-DIV-001 | D.10 | None | MISSING | Confirmed pivot with `pivot_at/confirmed_at` | `signals/pivots.py` | shared features | Critical causal | confirmation delay | 1 |
| SIG-DIV-002, SIG-DIV-003, SIG-DIV-004, SIG-DIV-005 | D.10 | None | MISSING | Four divergence types for three oscillators | `signals/divergence.py` | confirmed pivots | Critical causal | separation/trend/truncation | 2 |
| BBI-SIG-001, BBI-SIG-002, BBI-SIG-003 | D.11 | No BB or flow events | MISSING | Consume BB outputs read-only and permit method/quality constraints | `signals/flow_events.py` | MoneyFlowBlackbox | Critical semantic | method boundary/event causality | 3 |

### Data, DSL, UI and acceptance contracts

| Requirement ID | Spec section | Current implementation | Status | Gap / recommended action | Affected modules | Dependencies | Risk | Required tests | Suggested phase |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DATA-CANDLE-001 | G.1 | Unique four-column key exists | EXISTS_AND_COMPATIBLE | Preserve; add explicit indexes for dominant query shape if profiling justifies | candle model/migration | clean migration | Low | uniqueness/query plan | 0/5 |
| DATA-CANDLE-002 | G.1 | Import classifier validates; stored DB has two invalid Yahoo OHLC rows | EXISTS_NEEDS_EXTENSION | Run catalog-wide audit and quarantine/report invalid rows without rewriting silently | data quality/catalog | provenance policy | High | validation fixtures | data workstream |
| DATA-CANDLE-003 | G.1 | Missing sessions are inferred from calendar-day gaps | EXISTS_NEEDS_REFACTOR | Add exchange calendar/status distinction | market/data | lifecycle/calendar source | Critical | holiday/suspension/missing | data/4 |
| BT-TIME-001 | H.1 | Same-close signal and fill | EXISTS_NEEDS_REFACTOR | Signal cutoff T -> queued order -> T+1 open | backtest execution | pure order-intent model | Critical | TEST-BT-001 | 4 (before strategy use) |
| BT-TIME-002 | H.1 | No alternative availability model | MISSING | Keep extension point but do not add until specified | execution profiles | explicit spec | Low | profile validation | deferred |
| UI-SIG-001, UI-SIG-002, UI-SIG-003 | F.1 | Indicator catalog only | MISSING | Registry-driven signal catalog, defaults and experimental badges | frontend + signal API | stable registry | Medium | component/browser UAT | 5 |
| UI-EXP-001 | F.2 | No signal explanation UI | MISSING | Date/symbol explanation panel | frontend | result contract | Medium | deterministic UAT | 5 |
| TEST-SIG-001, TEST-SIG-002, TEST-SIG-003 | J.1 | Indicator tests exist; signal tests absent | MISSING | Determinism, warmup, reasons | backend tests | signals | High | exact named tests | 1-2 |
| TEST-CAUSAL-001, TEST-CAUSAL-002, TEST-CAUSAL-003 | J.1 | Replay slicing tests exist; signal future-invariance/pivot/new-high tests absent | MISSING | Shared truncation harness | backend tests | causal signals | Critical | exact named tests | 1-2 |
| TEST-ICHI-001 | J.1 | Tests assert future extension, not no-leak contract | EXISTS_NEEDS_REFACTOR | Golden causal displacement fixture and payload boundary | indicators/replay tests | contract decision | Critical | full vs truncated | 1-2 |
| TEST-BB-001, TEST-BB-002 | J.1 | None | MISSING | End-to-end method/quality and single BB authority | money flow tests | data mode | Critical | AT01-AT20 references | 3 |
| TEST-DSL-001 | J.1 | Malicious call and unknown-name tests exist | EXISTS_AND_COMPATIBLE | Expand attribute/import/comprehension/depth/size corpus; preserve whitelist | rule evaluator tests | none | Medium security | adversarial AST | 1 |
| TEST-BT-001, TEST-BT-002, TEST-BT-003, TEST-BT-004, TEST-BT-005 | J.1 | Current tests cover existing same-close/basic index T+2, not Master semantics | EXISTS_NEEDS_REFACTOR | Next-bar, settlement, holiday, limit-lock, phase-end golden fixtures | backtest tests | calendar/rules | Critical | exact named tests | 4 |
| TEST-MET-001, TEST-MET-002 | J.1 | Analytics tests exist; exact nine-metric conventions differ | EXISTS_NEEDS_REFACTOR | Golden ledger and N/A renderer | metrics tests | corrected ledger | High | zero/all-win/all-loss/breakeven | 4 |
| TEST-PERF-001 | J.1 | No phase call-count instrumentation | MISSING | Assert one feature computation per symbol/run | batch tests | phase runner | Medium | spy/counter | 4 |
| TEST-REPRO-001 | J.1 | Input hash and stable analytics partly tested | EXISTS_NEEDS_EXTENSION | Persist all versioned assumptions and rerun | backtest/history tests | BacktestRun model | Medium | stored rerun | 4 |

The Master Spec contains conceptual API/domain contracts but no additional concrete `API-*`, `DSL-*`, or `MET-*` numbered IDs beyond the explicit IDs above. Those conceptual requirements are covered in Sections 6, 8 and 9 of this report and must receive concrete acceptance IDs in each future ExecPlan.

## 5. Money Flow Data Capability Audit

The machine-readable artifact is [data_capability_matrix.csv](data_capability_matrix.csv).

### Production data facts

- Database size: 621,199,360 bytes.
- `candles`: 2,403,642 rows, 3,734 distinct symbols, overall timestamps from `2000-07-28` through `2026-06-23` (the overall maximum is one of three nonstandard `D/split` rows).
- Dominant segment: 2,398,650 `cafef`, `1D`, `unadjusted` rows for 3,731 symbols through `2026-06-19`.
- Yahoo segment: 4,989 `YahooFinanceFeed` rows for three symbols; timestamp convention differs and two rows fail OHLC envelope validation.
- Coverage is uneven: among `1D/unadjusted` symbol series, minimum 1 row, mean about 644, maximum 6,280; 463 symbols have fewer than 50 rows and 2,156 have fewer than 200.
- Production `import_runs` and `sync_runs` are both empty, so database rows cannot be traced to retained preview/manifests through those tables.
- No project-owned raw/cache market files were found outside dependencies, test fixtures and unrelated research repositories.

### Required field verdict

| Required field | Stored DB | CafeF parser | SSI adapter | vnstock adapter | Verdict |
| --- | --- | --- | --- | --- | --- |
| Trade timestamp/price/volume/value | No | No | No | No | INSUFFICIENT_DATA |
| Aggressor side | No | No | No | No | INSUFFICIENT_DATA |
| Historical best bid/ask | No | No | No | No | INSUFFICIENT_DATA |
| Matched value/volume separate from put-through | No | Parser discards extras | No | No | UNKNOWN upstream; unavailable in Sumi |
| Foreign buy/sell value | No | Parser discards extras | No | No | UNKNOWN upstream; unavailable in Sumi |
| Proprietary buy/sell value | No | Parser discards extras | No | No | UNKNOWN upstream; unavailable in Sumi |
| Buy/sell order counts/volume | No | Parser discards extras | No | No | UNKNOWN upstream; never treat as executed flow |
| Listing/delisting/suspension/security type history | No | No | No | No | INSUFFICIENT_DATA |
| Point-in-time index membership | No | No | No | No | INSUFFICIENT_DATA |

### Canonical capability classification

| Source / segment | Classification | Reason |
| --- | --- | --- |
| CafeF-labeled `1D/unadjusted` database history | OHLCV_PROXY | OHLCV exists. Exact matched value does not; use only estimated typical-price × volume after price-stream policy is frozen. |
| YahooFinanceFeed stored history | OHLCV_PROXY, DEGRADED | OHLCV only, inconsistent timestamp convention, two invalid OHLC rows and zero-volume observations. |
| Three `D/split/source=NULL` rows | INSUFFICIENT_DATA | Too sparse and unproven. |
| Named SSI/vnstock adapters | INSUFFICIENT_DATA | Synthetic generators, not actual provider observations. |
| EXECUTED_ORDER_FLOW | INSUFFICIENT_DATA | No executed-side data. |
| CLASSIFIED_ORDER_FLOW | INSUFFICIENT_DATA | No trades plus synchronized quotes or labeled validation sample. |
| TICK_TEST_ESTIMATE_RESEARCH_ONLY | INSUFFICIENT_DATA | No tick trades. |
| Whole-market BB and breadth | INSUFFICIENT_DATA | No point-in-time eligibility/exchange/lifecycle data. |

### Data Audit Gate decision

**NO-GO for production TRUE_FLOW.**  
**NO-GO for CLASSIFIED_ORDER_FLOW or tick-test research.**  
**Conditional GO for symbol-level OHLCV_PROXY research only**, after:

1. analysis-price adjustment semantics are frozen;
2. invalid/mixed candle streams are filtered;
3. proxy metadata always says `flow_method=OHLCV_PROXY` and `value_source=ESTIMATED_TP_X_VOLUME`;
4. outputs are not described as actual inflow/outflow; and
5. whole-market output remains disabled until a point-in-time universe exists.

## 6. Backtest Correctness Audit

| Area | Current behavior | Finding / required correction |
| --- | --- | --- |
| Signal timestamp | Snapshot includes close-derived indicator values at loop index `i` | Signal itself is known after close T; acceptable if availability is recorded as after close. |
| Entry timing/price | Rule true at T -> immediate `MARKET_AT_CLOSE`, explicit price `Close[T]` | **Critical violation.** Default must queue and fill no earlier than Open[T+1]. |
| Exit timing/price | Exit rule true at T -> immediate close at `Close[T]` | Same causal violation. |
| Stop loss / take profit | Strategy schema exposes percentages; backtest does not pass them into the trade or invoke replay auto-liquidation | Config is misleading/inert. Implement in a versioned execution state machine with deterministic same-bar precedence. |
| Trailing stop | Not present in backtest schema/path | Missing. |
| Position sizing | Fixed quantity or percent equity, with approximate fee-aware rounding to 100 shares | Fixed quantity validation occurs later. Percent sizing uses hard-coded fee and lifecycle allows negative practice cash. Move exact fee-aware sizing into backtest domain. |
| Round lot | 100 shares hard-coded in several paths | Needs `MarketRuleProvider(exchange,date,status)`; keep 100 only as a versioned default. |
| Commissions/tax | Buy 0.15%, sell 0.15%, sell tax 0.1%; executions store fee/tax | Formula aligns with current preset, but rates are global constants and not request/version configurable. |
| Cash accounting | Session cash is updated, but practice lifecycle explicitly does not block buys | Automated backtest must enforce allocated capital independently from practice UX. |
| Trade ledger | Entry/exit date/price, quantity, PnL and executions exist | Missing distinct signal/order/fill timestamps, settlement availability, exit reason and reliable `bars_held`. |
| T+ settlement | Backtest waits two observed rows; unused broker also uses bar-index difference; manual practice currently allows immediate close | No dated settlement lots, market calendar, PM-on-T+2 rule, or multi-lot FIFO. Separate practice and backtest profiles. |
| Holidays/calendar | Observed candle index acts as an implicit session list; gaps are only calendar-day warnings | Cannot distinguish holiday, suspension, delisting or missing vendor row. Required calendar/status provider absent. |
| Price bands/locked limits | Static exchange percentages exist; only limit-order request validation uses them | No tick-size rounding, date-versioned rules, liquidity/locked-limit rejection in active backtest. |
| Corporate actions | Adjustment type stored; no action ledger or split/dividend execution treatment | Backtest query can mix adjustment/timeframe and has no analysis/execution stream separation. |
| Phase boundaries | One date range only; open final position stays open and is absent from closed-trade analytics | No configured phase policy, unresolved-position output, or legally sellable phase-end rule. |
| Zero-trade metrics | Legacy response returns numeric zeros; typed metrics mark some values not applicable | Must implement exact nine-output N/A rules. |
| Winner convention | Legacy analytics uses `net_pnl > 0`; break-even is grouped with losses in places | Master requires `return_pct >= 0`; version and correct. |
| Bars held | `holding_days` is calendar-day difference; `holding_candles` is not populated in active path | Master requires exit bar index minus entry bar index. |
| Multi-ticker | Sequential calls, independent cash/session each | Semantically compatible but not batch-loaded and failures can leave partially persisted sessions. |
| Multi-phase | Not implemented | Must load warmup once, compute once, compile once, and slice execution ranges. |

### Look-ahead paths

1. Confirmed same-close execution in `BacktestService`.
2. Replay Ichimoku endpoint can serialize future-dated projected indicator rows because `IndicatorEngine._append_indicator_result` unions extended indexes.
3. Future signal work is exposed to pivot/divergence backdating unless `available_at` is first-class; no pivot implementation exists yet.
4. Scanner/backtest queries do not pin timeframe/adjustment type, allowing incompatible rows to mix.
5. Relative Strength global/replay benchmark query does not consistently pin adjustment type; backtest adapter supplies no benchmark dataframe, producing missing RS values rather than a valid comparison.

## 7. Performance Audit

### Measured snapshot

One ad-hoc warm-cache local run (diagnostic, not an SLA) measured:

- 50 per-symbol SQLite reads, 58,573 rows total: **264.55 ms**.
- One equivalent `IN (...)` read: **258.75 ms** (1.02× only on this warm local DB).
- Five indicators through `StrategyIndicatorAdapter` over 3,970 FPT rows: **169.78 ms**, returning 16 mapped series.
- 100,000 small DSL evaluations: **243.25 ms**.
- Current build output: ReplayPage chunk **175.70 kB** uncompressed; the date-related chunk is **178.69 kB**; total main index chunk **257.72 kB**.

The measurement suggests local SQLite query count is not yet the dominant cost for this dataset. Repeated dataframe copying/calculation scales more materially: five-indicator work at roughly 170 ms × 50 symbols is about 8.5 seconds before signal computation/execution/serialization, and would become much worse if repeated per phase.

### Bottleneck map

| Layer | Evidence | Priority / action |
| --- | --- | --- |
| DB I/O | Backtest/scanner issue one ORM query per symbol and materialize ORM objects; benchmark/regime causes extra queries | Medium. Batch-select columns after correctness; retain index. Current unique index covers the full candle key, while separate symbol/timestamp indexes also exist. |
| Indicator calculation | Adapter calls `IndicatorEngine.compute` per configured instance; each call copies the entire dataframe and recomputes dependencies | High. Build a dependency graph/shared feature frame; preserve independent display instances. |
| Signal calculation | Absent | Design vectorized category modules, not a monolith; materialize shared candle/volume/ATR features once. |
| Strategy evaluation | Python scalar snapshots and dict allocation per bar | Medium after correctness. Precompile validated AST/DSL and consume boolean arrays where possible. |
| Execution loop | Stateful Python loop; active backtest also performs database writes/queries/commits through service calls per fill | High. Use a pure in-memory execution kernel and persist the completed ledger transactionally. Numba only after profiling. |
| Multi-phase | Missing | Critical architecture requirement: never rerun indicator/signal stack per phase. |
| Serialization | Indicator APIs use dataframe `iterrows`; result payload returns every historical point and multiple series | Medium. Use column-oriented/vectorized conversion and pagination/compact contracts where appropriate. |
| Replay/frontend | One backend request and full visible-history recomputation per indicator instance on replay advance; duplicate instances repeat equivalent work | High for interactive use. Request coordination prevents stale rendering but not backend recomputation; add keyed feature cache/batched endpoint later. |
| Frontend rendering | Provider managers and request cancellation exist; bundle chunks are moderate but ReplayPage is largest feature chunk | Measure browser CPU/INP only after new panels exist; no evidence yet that chart rendering is the primary bottleneck. |

No optimization or Numba work should begin before a reproducible benchmark harness is added. The likely first wins are shared feature computation, pure ledger execution, compute-once phase slicing, and cache keys containing data/spec/method versions.

## 8. Proposed Target Architecture

```text
Stored/Provider Data
  -> MarketDataRepository + DataQuality/Lifecycle metadata
      -> IndicatorEngine (retain; numeric indicator authority)
      -> MoneyFlowBlackbox (new, independent, method-labeled)
          -> Shared Feature Matrix
              -> SignalEngine category modules + SignalRegistry
                  -> Safe StrategyRuleEvaluator (retain whitelist)
                      -> BacktestExecutionEngine (new pure domain kernel)
                          -> Trade/Settlement/Cash Ledger
                              -> BenchmarkMetricsAggregator
                                  -> Service/API persistence adapters
                                      -> React configuration/explanation/results UI
```

### Keep unchanged or nearly unchanged

- `IndicatorEngine` ownership and allow-listed indicator registry.
- The AST parser whitelist and declarative DSL operator set.
- ReplayService's server-side visible-candle slicing principle.
- Provider boundary DTO/interface and transactional import/sync staging pattern.
- Frontend chart managers, indicator instance state and drawing provider boundaries.
- Existing manual practice workflow as a separate, explicitly permissive practice profile.

### Extend

- Indicator definitions with version, warm-up, dependencies and causal/output metadata.
- Strategy namespace mapping so precomputed signals are safe identifiers.
- Run manifest with signal, BB, market-rule, dataset, adjustment, phase and execution versions.
- Data catalog with auditable provenance and DQ summaries.
- Frontend registry-driven configuration and explanation rendering.

### Refactor

- Extract automated execution from `TradeLifecycleService` into a pure `domain/backtest` kernel. The service should orchestrate persistence, not define fill semantics through replay side effects.
- Consolidate or retire the unused `domain/engine/broker.py` path after its useful ideas are migrated; do not maintain two divergent engines.
- Separate analysis price stream from execution price stream and require timeframe/adjustment selection in every query.
- Bound Ichimoku projected output with an explicit `projection=true` contract, never indistinguishable future-dated replay data.

### New boundaries

- `backend/app/domain/signals/`: registry, definitions/results, shared features, category modules.
- `backend/app/domain/money_flow/`: inputs, modes, calculator, aggregation/breadth and quality metadata only after gate decisions.
- `backend/app/domain/backtest/`: order intents, fills, positions, settlement lots, execution profile, metrics and batch runner.
- `backend/app/domain/market/`: calendar, settlement and date/exchange/security-status rules.

Money Flow outputs enter the feature matrix and are consumed read-only by Signal Engine/Strategy rules. They do not belong inside `IndicatorEngine`, and they never generate arbitrary BUY/SELL decisions themselves.

## 9. Detailed Implementation Plan

Each implementation batch must receive its own ExecPlan under `docs/exec-plans/`, list exact acceptance IDs, and stop on the escalation conditions in `AGENTS.md`.

| Task ID | Goal | Requirements covered | Expected files/modules | Dependencies | Tests | Definition of Done | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| AUD-CLEAN-01 | Establish a clean, reviewable baseline and reconcile current staged deletions/untracked V3 artifacts | G-01, repository DoD | Git/documentation only; no product edits | Owner decision on existing changes | clean-status inventory; retained evidence check | User-owned work is committed/stashed by owner; audit artifacts preserved; exact baseline commit recorded | High coordination |
| SIG-FND-01 | Versioned SignalRegistry, result/DQ contract, shared candle primitives, prior-volume baseline | FR-CORE-002/003/006/007, NFR-DET/DQ/MAINT, SIG-VOL-001 | new `domain/signals/{models,registry,candle_features}.py`; signal schemas/service/tests | IndicatorEngine retained | formula, zero-range, warmup, NaN, determinism, future-invariance | Registry metadata and feature result are deterministic and no trade/backtest imports exist | Medium |
| SIG-VOL-02 | Volume Spike end-to-end through API and minimal explanation UI | SIG-VOL-002, UI-SIG-001, UI-SIG-002, UI-SIG-003, UI-EXP-001, TEST-SIG-001, TEST-SIG-002, TEST-SIG-003 | `signals/volume.py`, signal API/service, frontend signal UI | SIG-FND-01 | thresholds, reasons, registry API, browser UAT 1440×1000 | User configures 2-3 params and sees value/reasons for visible replay date without future data | Medium |
| CAUSAL-03 | Shared truncation harness, confirmed pivots, new-high/low, SR primitives | FR-CORE-011, SIG-DIV-001, SIG-REG-006, SIG-SR-001, TEST-CAUSAL-001, TEST-CAUSAL-002, TEST-CAUSAL-003 | signals primitives/tests | SIG-FND-01 | append-future, pivot delay, current exclusion | Availability timestamps and truncation invariance proven | High |
| SIG-CORE-04 | Pattern and technical trigger catalog | SIG-PAT-001 through SIG-PAT-010; SIG-TECH-001 through SIG-TECH-006 | patterns/technical/registry/UI | FND/CAUSAL | formula/boundary/reasons/UAT | Registered, configurable, explained, causal signals | Medium |
| SIG-REG-05 | Regime, VSA, structure and Ichimoku signals | SIG-REG-001 through SIG-REG-005; SIG-VSA-001 through SIG-VSA-006; SIG-STR-001, SIG-HEALTH-001, SIG-ICHI-001 | regimes/vsa/structure/health/ichimoku | core features/SR | golden, zero-range, no double count, Ichimoku causality | All KEEP non-divergence signals satisfy contracts | High |
| SIG-DIV-06 | Four-way divergence for RSI/MACDH/Stochastic | SIG-DIV-002, SIG-DIV-003, SIG-DIV-004, SIG-DIV-005 | divergence/registry/UI | confirmed pivots | pairing/separation/trend/confirmation/truncation | Never backdated; explanations identify both pivots | High |
| BT-CAUSAL-01 | Pure causal next-session execution kernel using existing indicator/DSL strategies | BT-TIME-001, FR-CORE-005/011 | new `domain/backtest/{models,execution}.py`, service adapter | decision on execution profile | next-open, final-bar, fees, sizing, no same-close | Existing sample strategy runs produce versioned ledger with distinct signal/order/fill timestamps | Critical |
| MKT-RULE-02 | Versioned Vietnam calendar, settlement lots, lot/tick/band/locked rules | TEST-BT-002, TEST-BT-003, TEST-BT-004, Master sections 19/H | `domain/market/{calendar,settlement,rules}.py` | authoritative rule/calendar source | weekend/holiday/T+2 PM/locked/lot fixtures | Rule source/version documented; no calendar arithmetic | Critical/data |
| BT-BATCH-03 | Independent multi-ticker, multi-phase compute-once runner and phase policy | FR-CORE-008/009, NFR-PERF-001, TEST-PERF-001, TEST-BT-005 | batch runner/API/schemas | BT-CAUSAL, MKT-RULE | phase boundary, open/unsellable, call count | 30-50×3 benchmark harness, one feature calculation per symbol | High |
| MET-04 | Exact nine benchmark metrics and drill-down reconciliation | FR-CORE-010, TEST-MET-001/002 | `domain/backtest/metrics.py`, API/UI | authoritative trade ledger | zero/all winners/all losers/break-even/N/A/rounding | Exact raw numeric outputs; renderer-only rounding; table reconciles to ledger | High |
| DATA-BB-01 | Real provider/source inventory with retained samples and semantics | FR-CORE-012, BB Gate R0/R1 | source catalog, samples outside secrets, expanded matrix | provider credentials/licensing | schema/coverage/missingness audits | Each source×exchange×date gets canonical feasibility label | Blocker discovery |
| DATA-PIT-02 | Point-in-time symbol lifecycle/universe model | BB AT11, DATA-CANDLE-003 | domain/data models/migration/importers | authoritative lifecycle source | listing/delisting/suspension/membership fixtures | Survivorship-safe eligibility query works by date | Critical/data |
| BB-PROXY-03 | Research-only symbol OHLCV proxy under explicit method label | BB AT01/05-08/13/15/16/18/20 | `domain/money_flow` research module | adjustment/value-source decision | BB applicable ATs, corporate-action DQ | Bounded deterministic proxy, never mislabeled | High semantic |
| BB-TRUE-04 | Executed/classified flow prototype only if Data Gate passes | BB AT01-14/19/20 | money-flow adapters/classifier | real trade/quote data and labeled validation | coverage/agreement/offset sensitivity | Gate threshold documented and passed, otherwise task stops with rejection | BLOCKER-gated |
| BB-MKT-05 | Multi-horizon Market BB and breadth | BB AT09-12/17/19 | aggregation/breadth/API | DATA-PIT, acceptable flow primitives | aggregate identity, no score mean, PIT, breadth sums | Values aggregated before ratio; breadth separate; DQ visible | Critical |
| UI-INT-06 | Stable signal/BB/backtest configuration and reporting | UI contracts, FR-CORE-006/010/012 | frontend pages/components/APIs | frozen backend contracts | component tests and browser UAT at 1440×1000 and 1280×800 | Method/quality warnings, phases, explanations and exact metrics are usable | Medium |
| PERF-VAL-07 | Profile, cache, optimize and seal evidence | NFR-PERF-002, TEST-REPRO-001 | benchmark/cache/evidence docs | stable semantics | cold/warm benchmarks, cache invalidation, full gates/UAT | Measured target recorded; cache keys include all spec/data versions | Medium |

## 10. Decisions and Blockers

### BLOCKER

| Item | Evidence | Impact / required owner action |
| --- | --- | --- |
| True executed/classified Money Flow data | No qualifying fields, raw samples or real adapters | No TRUE_FLOW work. Data/provider owner must supply documented feed and sample coverage. |
| Point-in-time universe | Exchange and lifecycle metadata unavailable | No historical Market BB or breadth. Obtain/version lifecycle, status, security type and membership source. |
| Historical Vietnam market rules/calendar | No authoritative date-versioned source | Full backtest settlement/locked-limit/corporate-action correctness cannot be declared complete. Product/data owner must approve sources and conservative daily approximation. |
| Dirty baseline | Extensive pre-existing modified/deleted/untracked files | Do not start a write-heavy implementation batch until owner records the intended baseline; otherwise audit-to-diff traceability is weak. |

### NON_BLOCKING_OPEN_QUESTION

| Item | Recommendation |
| --- | --- |
| Ichimoku future projection in replay | Preserve causal cloud projection only behind an explicit projection contract/metadata and never mix it with visible candle timestamps. |
| Phase-end policy | Default to no forced liquidation; report open/unsellable positions separately until product owner freezes a benchmark policy. |
| Fee presets | Keep current rates as a named/versioned preset, not globals; do not claim historical universality. |
| Corporate-action price streams | Use adjusted analysis and unadjusted execution only after a corporate-action reconciliation policy is validated; otherwise degrade/disable affected runs. |
| BB 20/30/70/80 and confluence tolerance | Keep research parameters; do not optimize on P&L. |
| Existing synthetic providers | Rename visibly as demo providers immediately in the relevant future batch or replace through evidence-backed provider spikes; never present them as live connections. |
| Break-even winner convention | Adopt Master `return_pct >= 0` in a versioned metric implementation; preserve legacy analytics compatibility separately if needed. |

### RECOMMENDATION

- Keep `IndicatorEngine`; add a feature dependency layer around it.
- Keep the safe DSL whitelist; do not add arbitrary calls or `eval`.
- Separate manual practice and automated execution profiles.
- Use immutable in-memory backtest calculation followed by transactional persistence, not per-fill ORM service side effects.
- Add explicit timeframe/adjustment/source selection to scanner/backtest requests before broadening data use.
- Treat ADR-002 provider capability tables as prior research claims, not verified current implementation facts.
- Add evidence-quality assertions to browser UAT so a scenario cannot pass with evidence showing no state transition.

## 11. Proposed First Implementation Slice

### `SIG-FND-01 + SIG-VOL-02` — Explainable causal Volume Spike

This should be first because it is a small but complete vertical capability, uses the strongest existing asset (`IndicatorEngine`), establishes contracts needed by every later signal, has no dependency on unresolved TRUE_FLOW or Vietnam settlement data, and can be proven causal before the library expands.

**Exact scope**

1. Add versioned `SignalDefinition` and `SignalResult` models with parameters, dependencies, warm-up, `available_at`, score/reasons and data quality.
2. Add a modular `SignalRegistry` and a calculation service that consumes an already prepared OHLCV feature frame.
3. Implement shared candle primitives and `relative_volume = volume / mean(previous N volumes)`.
4. Implement `volume.spike` with `lookback`, `multiplier`, and optional direction confirmation.
5. Add replay-scoped signal registry/calculation API that never reads beyond `current_index`.
6. Add the smallest UI integration needed to configure the signal and display its deterministic explanation for the visible date.
7. Do **not** wire it into automated backtest execution in this slice; first correct `BT-TIME-001` in the following dedicated batch.

**Requirements:** FR-CORE-002, FR-CORE-003, FR-CORE-006, FR-CORE-007, FR-CORE-011, NFR-DET-001, NFR-MAINT-001, NFR-UX-001, NFR-DQ-001, SIG-VOL-001, SIG-VOL-002, UI-SIG-001, UI-SIG-002, UI-SIG-003, UI-EXP-001, TEST-SIG-001, TEST-SIG-002, TEST-SIG-003, TEST-CAUSAL-001.

**Expected files**

- New: `backend/app/domain/signals/models.py`, `registry.py`, `candle_features.py`, `volume.py`.
- New or extended service/API/schema modules for replay-scoped signal calculation.
- Focused backend tests under `backend/app/tests/`.
- Minimal frontend signal catalog/explanation components and tests.
- One ExecPlan under `docs/exec-plans/` before coding.

**Required tests**

- Exact prior-window formula and threshold boundaries.
- Warm-up, NaN, zero-baseline, zero-range and missing-volume behavior.
- Full-series versus every-prefix future-invariance.
- Determinism and params-hash/version stability.
- Registry whitelist and invalid-parameter rejection.
- Replay API proves no timestamp/value past `current_index`.
- UI explanation matches backend values/reason codes.
- Focused browser UAT at 1440×1000 plus full `verify-v2.ps1` and comprehensive UAT.

**Definition of Done**

- The user can configure Volume Spike with no source edits and inspect why it is active on the visible replay bar.
- Backend output is deterministic, versioned, causal and quality-aware.
- `IndicatorEngine` remains authoritative for indicators; Signal Engine owns interpretation; no trade execution dependency is introduced.
- All mapped acceptance IDs pass, screenshots/results are retained, production DB hash remains unchanged, and reviewer inspects diff/evidence.

## Verification evidence for this audit

- `scripts/verify-v2.ps1`: **PASS**.
  - Backend: 193 passed, one Starlette/httpx deprecation warning.
  - Alembic: upgraded a temporary database to `a2d639ecd5d5`.
  - Frontend: lint passed; 31 files / 193 tests passed; production build passed in 490 ms.
- Production database was inspected read-only and its SHA-256 matches the retained UAT report hash.
- Retained `test-results/comprehensive-uat/report.json`: 31/31 marked passed on 2026-09-12, but evidence-quality limitations noted above mean it is not acceptance evidence for the new Master Spec.
- Browser UAT was not rerun because this batch changed no product behavior or UI; the audit stops here as instructed.

---

**STOP:** This report completes discovery, audit, mapping and planning. No production implementation was started.
