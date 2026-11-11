# SUMI Final DEV Implementation Plan

> **Execution governance update — 2026-09-15:** Phase 1 is reviewer-sealed. The owner-approved autonomous program is `docs/dev-program/START.md`, with checkpoint `docs/dev-program/STATE.json`. On launch, its orchestrator may schedule the remaining roadmap as bounded independently reviewed batches, without human relay after each batch. It supersedes historical no-program-dispatch / per-batch STOP wording below, not contracts, numerical oracles, owner-only decisions or readiness/data gates. Bootstrap current checkout first; this documentation setup does not implement Phase 2. Historical planning status below is retained for provenance.

**Planning date:** 2026-09-12  
**Status:** consolidated roadmap, reviewed for bounded agent handoff; per-batch readiness gates still apply; no production implementation authorized  
**Scope:** Price, Volume, Money Flow, Signal/Strategy composition, causal Vietnam-aware backtesting, measurement validation, minimal Sumi validation UI, and migration readiness  
**Authoritative sources:** `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` for system scope; `SUMI_Money_Flow_Blackbox_V1_DataReady_Handoff_v3/` for Money Flow; verified Sumi/Doraemon audits for current capability

This document is the single program-level implementation plan. Each authorized implementation batch must still create a bounded ExecPlan under `docs/exec-plans/` using `PLANS.md`, identify exact acceptance IDs, and update its own progress/evidence. This document does not authorize code, schema, dependency, or database changes.

**Handoff rule (reviewed 2026-09-12):** Do not give an implementation agent the entire roadmap as an instruction to execute. Section P defines readiness gates, a bounded Phase 1 contract, numerical oracles, and a single-batch handoff template. These constrain the broader task cards below. The baseline freeze is the next preparatory task; it is not complete merely because HEAD has been recorded.

## A. Executive Decision Summary

### What Sumi is building

Sumi is a rapid, local-first quantitative research product for Vietnam equities. It will calculate three independent evidence dimensions—**Price**, **Volume**, and **Money Flow**—turn those measurements into versioned, explainable signals, compose signals through the existing safe strategy DSL, and evaluate strategies through a causal, reproducible, Vietnam-aware batch backtest.

Sumi is not the terminal production platform. Stable domain logic and contracts must be portable to Doraemon; Mizuhara will ultimately consume production APIs and provide the production frontend.

### Locked decisions

1. Retain backend `IndicatorEngine` as the standard-indicator authority and the AST/declarative `StrategyRuleEvaluator` as the safe composition boundary.
2. Add a dedicated `SignalEngine` and `SignalRegistry`; signal modules consume precomputed primitives and emit deterministic values, reasons, versions, quality and availability time. They never execute trades.
3. Separate automated backtest execution from the permissive manual-practice lifecycle. A close-derived daily signal at T can normally fill no earlier than Open[T+1].
4. Build independent single-asset batch tests: each ticker gets configured capital; phases reuse one feature calculation per symbol. This is not a shared-cash portfolio simulator.
5. Use the exact v3 BB calculation methods: `ACTIVE_TRADE_FLOW_VALUE`, `ACTIVE_TRADE_FLOW_VOLUME`, `ACTIVE_VOLUME_VALUE_ESTIMATE`, `CLASSIFIED_TRADE_FLOW`, and `OHLCV_PROXY`.
6. Current historical Symbol BB is `OHLCV_PROXY`. No audited source qualifies today for production active/executed or classified flow. KBS session ticks remain `TICK_TEST_ESTIMATE_RESEARCH_ONLY` acquisition evidence, not a production BB method.
7. Never splice methods anonymously. Every BB observation carries source, method, method version, value source, coverage and quality.
8. Treat Money Flow BB as a measurement subsystem. BUY/SELL/WATCH belongs to registered signals and strategy rules.
9. Define Sumi V1 market scope as a versioned curated `SUMI-420`, not all Vietnamese securities since 2000. A name/version does not manufacture valid historical membership.
10. Keep `50` as the only structurally fixed BB level. Levels 20/30/70/80, turn/confluence tolerances and labels such as Money Strong/Saturated remain research parameters.
11. Do not add ML signals, an indicator-soup BB, horizon-to-investor-class inference, or geometric support/resistance curvature in V1.
12. Build only the Sumi UI required to configure, inspect and validate signals, strategies, backtests and BB measurements.

### What can be implemented now

- Signal contracts/registry, shared causal features, Volume Spike and the full explainable Price/Volume signal library.
- A pure causal backtest kernel, versioned execution assumptions, exact nine metrics, and compute-once multi-ticker/multi-phase orchestration.
- Provider-neutral Money Flow contracts and Symbol `OHLCV_PROXY` using audited daily bars.
- BB measurement-validation tooling using method/value-source/quality segments.
- SUMI-420 and Market BB/Breadth calculation frameworks exercised on fixtures and explicitly retrospective research universes.

### What remains research-only or data-blocked

- KBS tick-test and `LC`/matched-by-price semantics; SSI, vnstock_data, FireAnt and FiinQuant trials.
- Production `CLASSIFIED_TRADE_FLOW` and all active-flow methods until semantics, history, coverage, access stability and license gates pass.
- Published historical SUMI-420 Market BB/Breadth before a version/effective-date membership dataset and coverage policy are approved.
- Final BB event thresholds, technical-health weights, phase-end liquidation default, historical market-rule source and corporate-action treatment.

## B. Requirement Scope Matrix

| Capability | Classification | V1 interpretation / gate |
| --- | --- | --- |
| Shared candle/price primitives | `IMPLEMENT_NOW` | Causal reusable frame; zero-range, warmup and quality semantics |
| Bullish/bearish candle patterns | `IMPLEMENT_NOW` | Master `SIG-PAT-001..010`; familiar deterministic patterns only |
| Bullish/bearish technical triggers | `IMPLEMENT_NOW` | `SIG-TECH-001..006`; crosses/transitions plus explainable minimum confirmations |
| Volume / Relative Volume / Volume Spike | `IMPLEMENT_NOW` | `SIG-VOL-001..003`; baseline excludes current bar |
| Trend/regime: Up/Down/Sideways/Correction/Recovery/New High/Low | `IMPLEMENT_NOW` | `SIG-REG-001..006`; candidate defaults are versioned, not P&L-fitted |
| Simplified VSA and support/resistance | `IMPLEMENT_NOW` | `SIG-VSA-001..006`, `SIG-SR-001`; explicitly technical inference |
| Good/Bad candle structure | `IMPLEMENT_NOW` | `SIG-STR-001`; transparent score and reasons |
| Technical Health | `IMPLEMENT_NOW` | `SIG-HEALTH-001`; 3–4 distinct families; BB absence is not negative evidence |
| Ichimoku bullish/bearish | `IMPLEMENT_NOW` | `SIG-ICHI-001`; resolve projected-index boundary and prove future invariance |
| Four-way divergence | `IMPLEMENT_NOW` | `SIG-DIV-001..002`; confirmed pivots, emit at `confirmed_at`, never backdate |
| Signal Engine / Registry / explanations | `IMPLEMENT_NOW` | Required common contract and stable namespaces |
| Safe Strategy DSL | `IMPLEMENT_NOW` | Extend identifiers/adapters only; retain AST whitelist and reject arbitrary calls |
| Causal daily backtest | `IMPLEMENT_NOW` | Replace same-Close[T] behavior with versioned next-valid-event execution |
| Vietnam execution/calendar/settlement | `IMPLEMENT_NOW` | Framework plus approved conservative daily profile; historical source completeness remains gated |
| Independent multi-ticker/multi-phase runner | `IMPLEMENT_NOW` | Compute once per symbol, slice phases, independent capital |
| Exact nine metrics / ledger reconciliation | `IMPLEMENT_NOW` | Master signs, break-even and `N/A` semantics |
| Symbol BB historical | `IMPLEMENT_NOW` | `OHLCV_PROXY` only; actual/estimated value source explicit per segment |
| BB active-flow fixtures/CSV adapter | `IMPLEMENT_FRAMEWORK_ONLY` | Proves source-independent formula without claiming real capability |
| KBS tick-test | `RESEARCH_IN_PARALLEL` | Session-only; side ignored until validated; never surfaced as production True Flow |
| Production True/Active Flow | `BLOCKED_BY_DATA` | Needs accepted source semantics, coverage, stability and license |
| BB turn/confluence/business-state thresholds | `RESEARCH_IN_PARALLEL` | Measurement validation before predictive testing; 50 alone fixed |
| SUMI-420 version model and fixture aggregation | `IMPLEMENT_FRAMEWORK_ONLY` | Versioned immutable universe artifacts and gates can be built now |
| Approved `SUMI420_v1` membership/effective date | `RESEARCH_IN_PARALLEL` | Product/data decision plus historical eligibility evidence |
| Published Market BB / Flow Breadth | `BLOCKED_BY_DATA` | Requires applicable universe version, comparable primitives and coverage thresholds |
| Signal/strategy/backtest/BB APIs | `IMPLEMENT_NOW` | Vendor-neutral, versioned, causal boundaries only |
| Sumi validation UI | `IMPLEMENT_NOW` | Minimum catalog, explanations, phase results and BB validation views |
| Production collectors/raw trade store | `DEFER` | Doraemon-owned after a source passes access/license gates; not Sumi MVP infrastructure |
| RPA data collection | `DEFER` | Use only after API/export paths fail and terms permit it |
| Full historical Vietnam-market reconstruction | `DEFER` | Not required for SUMI-420 MVP; retain capability for future universes |
| Shared-cash portfolio optimization | `DEFER` | V1 is batch single-asset testing |
| Geometric “uốn hỗ trợ/kháng cự” | `DEFER` | May return only under a separate simple-composite specification |
| ML/opaque scores, institution/retail inference from horizons, indicator-soup BB | `PROHIBITED` | Violates explainability and semantic constraints |

## C. Decision Register

| Decision | Reason | Authoritative source | Implementation consequence | Reconsideration condition |
| --- | --- | --- | --- | --- |
| Sumi researches; Doraemon owns production data; Mizuhara owns production UX | Avoid throwaway infrastructure while keeping iteration fast | Product brief; Doraemon audit | Pure calculations and DTOs; minimal Sumi-specific persistence/UI | Product/platform ownership formally changes |
| Keep `IndicatorEngine` | Real registry-backed 16-indicator backend authority exists | Sumi Master; current code/audit | Signals request/reuse indicator outputs; no duplicate indicator formulas | A proven correctness blocker requires an ADR |
| Add separate Signal Engine/Registry | Current code has no reusable explanations/version/availability contract | Master FR-CORE-003/006/007; audit | New modular signal domain; no trade imports | Never, absent a replacement architecture decision |
| Keep safe AST/declarative evaluator | Existing whitelist rejects calls/attributes and already supports needed basic operators | FR-CORE-004; current code | Map registered outputs to allowed identifiers; do not add runtime functions casually | A bounded operator has tests and cannot be precomputed cleanly |
| Automated execution is a pure domain kernel, separate from manual practice | Current backtest writes through `TradeLifecycleService` and inherits permissive practice semantics | BT-TIME-001; Sumi audit | Signal/order/fill timestamps, settlement lots and ledger are explicit; persistence follows calculation | Only if practice and backtest profiles become provably equivalent |
| Default Close[T] signal → Open[T+1] fill | Current same-close path is known non-causal | Master H.1; TEST-BT-001; audit | Pending order intent processed at next valid execution event | Separate intraday availability model is specified/tested |
| v3 BB `flow_method` enum supersedes older Money Flow method names | v3 is specialized latest authority | BB DataReady v3; brief precedence | Use v3 enums in BB domain/API; version schema | New BB ADR/version |
| Retain audit capability classifications separately | `EXECUTED_ORDER_FLOW` describes source evidence; v3 `ACTIVE_TRADE_FLOW_VALUE/VOLUME` describes normalized calculation input | Doraemon audit + BB v3 | `capability_class` and `flow_method` are distinct fields; source qualification maps between them | Unified vocabulary is formally versioned across Sumi/Doraemon |
| Historical Symbol BB uses `OHLCV_PROXY` now | Only verified historical daily OHLCV exists broadly | Doraemon audit | Implement proxy formula, lineage and value-source segmentation; no True Flow language | Accepted active-flow history passes gate |
| KBS side labels remain `UNKNOWN` | Arithmetic consistency does not prove aggressor semantics | Doraemon audit | Tick-test may ignore `LC`; no production active/classified method | Contract or independent labeled-validation threshold passes |
| Do not adopt Doraemon `money_flow_daily_v1` as BB | It is OBV/MFI/AD/CMF/turnover composite and explicitly `not_qmv_blackbox` | Doraemon code/API audit; BB v3 | Keep distinct product/method; one authoritative BB formula in Sumi | BB specification deliberately changes |
| BB v3 tables are target concepts, not mandatory Sumi MVP tables | Sumi is research product; current sources do not justify raw stream storage; Doraemon is future owner | BB v3 database spec reconciled with Master E.8 and product purpose | Start with DTOs, immutable fixtures/config and run artifacts; add schema only on demonstrated query/recovery need | An authorized collector or multi-user persistence use case passes its gate |
| No anonymous method/value-source splice | Methods have different meanings and missing actual value is not zero | BB v3; audits | Series are segmented and API/UI expose method/value source/coverage | Never; only explicit comparison views may align segments |
| SUMI-420 narrows product scope but does not erase PIT requirements | A curated version avoids full-market reconstruction; historical validity still needs applicable membership evidence | BB v3 08; Master G.4; Doraemon audit | Framework now; publish only from version effective date or with explicit retrospective/survivorship label | Valid dated membership/lifecycle data expands coverage |
| Fixed-universe historical research is allowed only as labeled retrospective analysis | Useful for method validation but not a historical “market” claim | Reconciliation decision | Use `universe_mode=RETROSPECTIVE_FIXED`, show survivor caveat, exclude from canonical Market BB acceptance | Membership is validated for each calculation date |
| Measurement validation precedes predictive/P&L validation | BB thresholds/states are not frozen | BB v3; brief | Distribution/behavior reports first; no threshold selection from Net Profit | Measurement acceptance is documented and thresholds are versioned |
| UI remains validation-focused | Mizuhara is future production frontend | Product purpose | Extend existing pages minimally; avoid a new design system or vendor-aware UI | Sumi becomes an approved production UI |

### Conflict reconciliation

- **Older Master vs latest BB enum:** Master names such as `EXECUTED_ORDER_FLOW` and `TICK_TEST_ESTIMATE` remain useful source-capability labels. BB v3 wins for calculation `flow_method`; a qualified direct executed source maps to `ACTIVE_TRADE_FLOW_VALUE` or `ACTIVE_TRADE_FLOW_VOLUME` according to the actual measure.
- **Older BB documents vs v3:** v3 formulas, fallback ladder, `SUMI-420` and build-now/plug-data-later contracts supersede earlier Money Flow detail. Older documents remain historical rationale only.
- **BB v3 candidate sources vs Doraemon audit:** SSI/vnstock_data/FireAnt/FiinQuant are research candidates, not installed/verified project capabilities. KBS daily/history is verified; KBS active-side semantics are not.
- **Full-market PIT blocker vs SUMI-420:** the earlier blocker applied to a claim about the full historical Vietnam market. SUMI-420 reduces the required membership dataset to one managed universe, but canonical historical Market BB/Breadth still requires a version applicable on each date. A current list backfilled into the past is retrospective research, not PIT truth.
- **v3 “database schema now” vs Sumi portability:** domain contracts, adapters and formula tests are implemented now; vendor raw/canonical production stores remain Doraemon-owned unless a Sumi collector is explicitly authorized. This follows Master E.8's selective persistence rule.

## D. Verified Current-State Architecture

### Freshness check on 2026-09-12

The current checkout still has no `backend/app/domain/signals/`, Money Flow/Blackbox domain, multi-phase runner, exact benchmark-metrics module, or date-versioned market-rule provider. `BacktestService` still evaluates Close[T] and submits `MARKET_AT_CLOSE` on T, still uses an index-distance T+2 guard, and still ignores configured stop-loss/take-profit in its execution loop.

Changes newer than the initial audit are user-owned and must be preserved:

- `strategy_indicator_adapter.py` now maps raw OHLCV arrays and named Ichimoku columns into strategy inputs.
- New strategy examples/tests cover EMA crossover and Ichimoku input mapping.
- `StrategyLabPage.tsx` has quick symbols/date presets, a multi-strategy equity chart and P&L-derived ratings/recommendations.
- These changes improve current UX/strategy inputs but do not create the requested signal registry, causal execution, multi-phase semantics or exact nine metrics. P&L-derived recommendation language must not be reused to calibrate BB measurement thresholds.

### Module disposition

| Actual module/surface | Current role | Disposition |
| --- | --- | --- |
| `backend/app/domain/engine/indicator_engine.py` | Registry-backed numeric indicators | **KEEP/EXTEND metadata only**; remain authoritative |
| `backend/app/domain/engine/strategy_indicator_adapter.py` | Maps configured indicators/raw OHLCV to arrays | **EXTEND** into shared feature-frame adapter; preserve recent Ichimoku/OHLCV work |
| `backend/app/domain/strategy/rule_evaluator.py` | AST whitelist and declarative operator evaluator | **KEEP**; add registered identifiers without widening syntax |
| `backend/app/domain/strategy/strategy_rule_evaluator.py` | Validates/evaluates snapshots | **EXTEND/ADAPT** to precomputed signal arrays and compiled rules |
| `backend/app/domain/strategy/strategy_schema.py` | Indicator/rule/sizing/risk config | **EXTEND** with strict resolved signal/version and execution-profile contracts |
| `backend/app/services/backtest_service.py` | Orchestrates data, indicators, scalar loop, replay persistence/analytics | **REFACTOR** to orchestration only after pure kernel exists |
| `backend/app/services/trade_lifecycle_service.py` | Manual replay/practice decisions | **KEEP for practice**; prohibit from automated kernel |
| `backend/app/domain/engine/broker.py` and related engine models/events | Alternative orphan simulation ideas | **REVIEW/RETIRE OR EXTRACT**; do not operate two engines |
| `backend/app/services/analytics_service.py` | Rich existing analytics | **KEEP for legacy analytics; EXTEND via separate exact benchmark aggregator** |
| `backend/app/models/{candle,symbol,replay_session,trade,execution,position,order}.py` | Current market/practice persistence | **REUSE cautiously**; do not make domain calculations query ORM |
| `backend/app/models/strategy_lab_run.py` | JSON request/result/metrics history | **REUSE for MVP run artifacts** before adding normalized run tables |
| `backend/app/api/{indicators,backtest,strategy_lab,replay}.py` | Existing APIs | **EXTEND with versioned schemas**; keep legacy routes compatible during transition |
| `frontend/src/pages/StrategyLabPage.tsx` | Existing comparison/sweep UI | **EXTEND minimally**; remove no user-owned work; add phase/method/quality semantics after APIs freeze |
| Replay/chart/indicator frontend modules | Visible-history and plotting foundations | **KEEP/EXTEND** for signal/BB overlays through existing boundaries |
| Signal Engine/Registry | Absent | **NEW MODULE REQUIRED** |
| Money Flow BB | Absent | **NEW MODULE REQUIRED** |
| Pure backtest/market-rule/metrics domain | Absent | **NEW MODULE REQUIRED** |
| SUMI-420 version artifact/resolver | Absent | **NEW MODULE REQUIRED** |

## E. Target Domain Architecture

```text
Provider APIs / Sumi candles / immutable research files
                       │
                       ▼
              Market Data Port
       canonical bars + source/DQ/as-of
          ┌────────────┴────────────┐
          ▼                         ▼
 Indicator Engine            Money Flow subsystem
 numeric primitives        normalized input → BB series
          └────────────┬────────────┘
                       ▼
              Shared Feature Matrix
                       ▼
          Signal Engine + Signal Registry
       values/states/reasons/available_at/version
                       ▼
            Safe Strategy Rule Evaluator
               precomputed boolean arrays
                       ▼
       Backtest Execution Engine + MarketRuleProvider
         order intents → fills → settlement/cash ledger
                       ▼
                Metrics Aggregator
                       ▼
             Services / API / Run Artifact
                       ▼
           Minimal Sumi validation UI / export
```

### Boundary responsibilities

| Boundary | Owns | Must not own |
| --- | --- | --- |
| Market Data Port | Canonical bar/flow/universe reads, source lineage, as-of and quality | Provider-specific payloads above adapters; indicator/signal math |
| Indicator Engine | Standard numeric indicators and indicator metadata | Signal meaning, BB formula, DB/API queries |
| Signal Engine/Registry | Atomic/composite states, parameter validation, reasons, versions, causal availability | Trade execution, vendor access, arbitrary runtime functions |
| Money Flow subsystem | v3 input contracts, five flow methods, Symbol/Market formulas, quality/method boundaries | KBS/SSI clients, BUY/SELL advice, MFI/OBV composite substitution |
| SUMI-420 Universe | Immutable versions, effective intervals, eligibility/coverage resolution | Silent current-survivor backfill |
| Strategy DSL | Safe composition of precomputed identifiers | Indicator calculation, unrestricted calls, execution side effects |
| Backtest Execution Engine | Orders, next-event fills, cash, positions, settlement lots, exits and phase policy | Indicator/signal recomputation, UI formatting, practice workflow shortcuts |
| Trade Ledger | Reproducible signal/order/fill dates, fees/tax, quantities and realized outcomes | Summary metric rounding |
| Metrics Aggregator | Exact nine metrics from closed trades and explicit open-position diagnostics | Trade simulation or inferred fills |
| API layer | Validation, orchestration, versioned DTO serialization | Core formulas or vendor payload leakage |
| UI | Configuration, explanations, warnings, charts, result drill-down/export | Independent formulas or hidden quality/method conversion |

### Prohibited dependency directions

- BB Engine → KBS/SSI/FireAnt/vnstock_data client.
- Signal Engine → FastAPI, SQLAlchemy, TradeLifecycleService or execution kernel.
- Backtest kernel → IndicatorEngine/SignalEngine calculation or database session.
- Domain formulas → Sumi SQLite models.
- UI → vendor payloads, provider secrets or independently recomputed indicators/signals/metrics.
- Market BB → average of symbol BB values or index volume substitution.
- Automated backtest → permissive manual-practice execution semantics.

## F. Detailed Phase Plan

### Phase overview

| Phase | Outcome | Primary gate |
| --- | --- | --- |
| 0 | Reproducible baseline, compatibility snapshots and causal test harness | Dirty-checkout attribution and prefix-invariance tests are recorded |
| 1 | Reusable Signal Engine plus an explainable causal Volume Spike vertical slice | Registry, reasons, availability and browser evidence pass |
| 2 | Pure next-event backtest kernel with explicit Vietnam market-rule profiles | Signal/order/fill dates, settlement and ledger tests pass |
| 3 | Price, Volume, VSA and support/resistance signal catalog | Each signal has causal, numerical and explanation parity evidence |
| 4 | Compute-once multi-ticker/multi-phase runner and exact nine metrics | Standalone/batch parity and closed-trade metric fixtures pass |
| 5 | Method-aware Symbol Technical Flow BB using audited `OHLCV_PROXY` data | Formula, lineage, fallback and no-splice tests pass |
| 6 | Measurement validation and threshold/version decision package | Behavioral evidence is accepted independently of P&L |
| 7 | Versioned SUMI-420 framework; gated Market BB and Breadth | Membership, comparable input and coverage gates pass |
| 8 | Remaining advanced signals, Flow events and strategy integration | Availability, causal and semantics tests pass per family |
| 9 | Minimal research-validation UI across signals, backtests and BB | Full browser UAT and reviewed 1440×1000 evidence pass |
| 10 | Performance, observability, reproducibility and extraction readiness | NFR budgets, golden fixtures and migration parity pass |

### Universal batch gate

Every task below is a separate bounded batch unless its ExecPlan proves two adjacent tasks form one vertical capability within 3–10 working days. Before coding, record baseline commit/status, scope, acceptance IDs, rollback and exact commands. Every batch runs focused tests plus `./scripts/verify-v2.ps1`; user-facing chart/UI work must also run `./scripts/run-comprehensive-uat.ps1` and retain reviewed 1440×1000 evidence. Tests and UAT must use temporary databases, never `backend/sumi.db`. A reviewer inspects diff, evidence and deviations before completion.

### Phase 0 — Baseline, contracts and causal harness

#### `P0-BASE-01` — Freeze an implementation-safe baseline

- **Goal:** preserve the heavily dirty user-owned checkout and define compatibility seams before parallel work begins.
- **Requirements:** FR-CORE-014, NFR-DET-001, NFR-OBS-001, NFR-COMP-001, TEST-REPRO-001.
- **Dependencies:** this final plan; user authorization of a DEV task. No branch/worktree unless explicitly requested.
- **Expected modules/files:** one batch ExecPlan; current git inventory; API/schema compatibility fixtures. No product module is created merely for scaffolding.
- **Database impact:** none.
- **API impact:** none; compatibility snapshots only.
- **UI impact:** none; UI behavior is captured only as baseline evidence.
- **Tests:** record current focused/full gate results, DB hash, route/schema snapshots and representative strategy outputs.
- **Acceptance / DoD:** exact baseline and user-owned modifications are recorded; rollback is possible; known current failures are not hidden; no unrelated change enters the batch.
- **Risk:** coordination conflicts in the dirty checkout.
- **Migration:** establish contract snapshots that later protect Doraemon/Mizuhara integration.

#### `P0-CAUSAL-02` — Shared future-invariance and availability test harness

- **Goal:** provide reusable prefix-vs-full and `available_at` assertions before adding signals.
- **Requirements:** FR-CORE-011, TEST-CAUSAL-001/002/003, TEST-ICHI-001.
- **Dependencies:** P0-BASE-01.
- **Expected modules/files:** focused backend test helpers/fixtures under the existing test tree; no production behavior change.
- **Database impact:** none.
- **API impact:** none.
- **UI impact:** none.
- **Tests:** future-row append/mutation, warmup, intentional pivot delay, projected Ichimoku separation.
- **Acceptance / DoD:** a feature can declare immediate or delayed availability and the harness detects backdating/future mutation.
- **Risk:** false positives for intentionally projected display data; solve with explicit projection metadata.
- **Migration:** pure fixtures/assertions can move with domain modules.

### Phase 1 — Signal foundation and first vertical capability

#### `P1-SIG-01` — Signal models, registry and shared features

- **Goal:** introduce the reusable contract behind every Price/Volume/Flow-derived signal.
- **Requirements:** FR-CORE-001/002/003/006/007/011; NFR-DET-001, NFR-DQ-001, NFR-MAINT-001, NFR-UX-001; TEST-SIG-001/002/003.
- **Dependencies:** P0-CAUSAL-02; retain IndicatorEngine.
- **Expected modules/files:** new candidate package `backend/app/domain/signals/` for models, registry and candle features; signal Pydantic schemas/service; focused tests. Exact filenames are frozen in the batch ExecPlan after checking for equivalent modules.
- **Database impact:** none; definitions are code/version controlled and outputs transient.
- **API impact:** registry and replay-scoped calculation contracts may be introduced behind explicit schemas.
- **UI impact:** none until P1-SIG-02.
- **Tests:** zero-range candles, missing/NaN/warmup, deterministic `params_hash`, invalid parameters, stable namespaces, no domain import of execution/API/ORM.
- **Acceptance / DoD:** registry returns name/category/version/dependencies/defaults/warmup/delay/reason schema; feature frame excludes current volume from prior baseline; outputs carry quality and availability.
- **Risk:** over-generalized framework. Limit abstractions to the first signal plus known contracts.
- **Migration:** pure dataclasses/Pydantic-free core types where practical; thin Sumi schema adapters.

#### `P1-SIG-02` — Explainable causal Volume Spike end to end

- **Goal:** let a user configure and inspect `volume.spike` on the visible replay bar.
- **Requirements:** SIG-VOL-001/002, FR-CORE-004, UI-SIG-001/002/003, UI-EXP-001, TEST-SIG-001/002/003, TEST-DSL-001, TEST-CAUSAL-001.
- **Dependencies:** P1-SIG-01 and ReplayService current-index boundary.
- **Expected modules/files:** volume signal module; signal service/API/router; minimal frontend signal catalog/explanation component and API types; tests.
- **Database impact:** none.
- **API impact:** registry plus replay-scoped signal calculation; response includes values/reason codes/intermediates/params/version/quality/`available_at` and never future rows.
- **UI impact:** configure only period and multiplier in the first slice; display relative volume, threshold and warmup/quality. Optional direction/ATR filters are deferred to a later explicitly scoped extension.
- **Tests:** exact prior-N formula, equality boundary, zero baseline, missing volume, every-prefix invariance, invalid config, response timestamp cap and component/browser explanation parity.
- **Acceptance / DoD:** user-visible vertical capability passes focused/full gates and 1440×1000 UAT; isolated DSL resolution passes with precomputed signal fixtures and quality guards. No production strategy persistence, backtest execution or P&L evaluation is included.
- **Risk:** accidentally presenting a signal as volume “flow.” Labels must remain participation/activity.
- **Migration:** registry/result contract is the template for Doraemon domain adoption and Mizuhara rendering.

### Phase 2 — Causal automated execution foundation

#### `P2-BT-01` — Pure next-event backtest kernel

- **Goal:** eliminate same-Close[T] execution and per-fill practice-service side effects.
- **Requirements:** FR-CORE-005/011/014, BT-TIME-001/002, TEST-BT-001, NFR-DET-001.
- **Dependencies:** P0 harness; existing indicator/DSL arrays. It need not wait for the full signal catalog. Promote the P1 signal binding/quality adapter into the new execution path only after timing/ledger tests pass; do not wire it into the old backtester first. This basic integration belongs to P2, not P8.
- **Expected modules/files:** new candidate `backend/app/domain/backtest/` models/execution kernel; refactor `backtest_service.py` to orchestration; review/extract or retire `domain/engine/broker.py`; compatibility tests.
- **Database impact:** no migration initially; calculate in memory and persist a completed result artifact transactionally.
- **API impact:** existing `/api/backtest/run` remains compatible while exposing versioned execution assumptions/warnings.
- **UI impact:** show signal/order/fill dates and profile warnings; no redesign.
- **Tests:** Close[T]→Open[T+1], final-bar signal, fee-aware lot sizing, insufficient cash, deterministic simultaneous exits, no domain DB writes, legacy request compatibility.
- **Acceptance / DoD:** default daily engine cannot fill on the signal close; ledger separates signal/order/fill; failed runs leave no partial backtest sessions/trades.
- **Risk:** legacy result drift. Keep a documented compatibility layer and version the execution model.
- **Migration:** pure kernel and ledger migrate to Doraemon; Sumi service/persistence adapters do not.

#### `P2-MKT-02` — Vietnam market-rule profile

- **Goal:** model trading sessions, settlement lots, fees/tax, lot/tick/price-band and locked-fill policies outside the execution loop.
- **Requirements:** Master sections 19/H; TEST-BT-002/003/004; DATA-CANDLE-003.
- **Dependencies:** P2-BT-01 contracts; approved source and conservative assumptions.
- **Expected modules/files:** new candidate `backend/app/domain/market/` calendar, settlement and rule-provider modules; named fixture/profile data; tests.
- **Database impact:** none for V1; immutable versioned rule/calendar resources. Add tables only if runtime editing/query requirements emerge.
- **API impact:** request names an `execution_profile`, `market_rule_version`, fee/tax/slippage profile; response echoes resolved assumptions.
- **UI impact:** select named profile and view limitations, not dozens of raw fields.
- **Tests:** weekend/holiday settlement, T+ sellability, settlement-day daily ambiguity, lot rounding, price/tick validation, locked ceiling/floor and missing/suspended bar.
- **Acceptance / DoD:** no calendar-day T+ arithmetic or scattered constants; conservative behavior and source/version are explicit.
- **Risk:** historical Vietnam rule changes and incomplete corporate-action data. Degrade affected runs rather than fabricate precision.
- **Migration:** Doraemon eventually owns authoritative calendars/rules; portable resolver contracts remain shared.

### Phase 3 — Core Price/Volume signal library

#### `P3-PRICE-01` — Patterns, technical triggers and regimes

- **Goal:** implement the primary Price dimension as registered causal signals.
- **Requirements:** SIG-PAT-001..010, SIG-TECH-001..006, SIG-REG-001..006, SIG-SR-001, TEST-CAUSAL-003.
- **Dependencies:** P1 foundation and P0 causal harness; IndicatorEngine EMA/ATR/ADX/BB/MACD/RSI.
- **Expected modules/files:** signal category modules for patterns, technical triggers, regimes and support/resistance; registry entries, schemas/tests and incremental UI forms.
- **Database impact:** none.
- **API impact:** existing signal calculation contract accepts new definitions; no new per-signal route.
- **UI impact:** registry-driven forms with defaults/presets and explanations; advanced parameters hidden.
- **Tests:** every formula/boundary, prior-window exclusion, crosses vs persistent state, correction/recovery context, warmup/NaN, future invariance, selected-child reasons.
- **Acceptance / DoD:** all registered signals are deterministic, versioned, have 2–5 ordinary parameters and satisfy focused browser evidence where charted.
- **Risk:** too many signals in one batch. ExecPlan may split patterns and regimes while retaining complete vertical outcomes.
- **Migration:** category modules remain pure; UI metadata drives both Sumi and future Mizuhara forms.

#### `P3-VSA-02` — Simplified VSA and candle structure

- **Goal:** implement Price+Volume technical inferences without claiming observed supply/demand or investor identity.
- **Requirements:** SIG-VOL-003, SIG-VSA-001..006, SIG-STR-001, SIG-SR-001.
- **Dependencies:** P3-PRICE-01 support/resistance; P1 shared volume/candle features; ATR.
- **Expected modules/files:** VSA and structure signal modules, registry/tests, minimal explanation UI additions.
- **Database impact:** none.
- **API impact:** registry/calculation only.
- **UI impact:** reasons show range/ATR, close location, relative volume and matched prior support/resistance source.
- **Tests:** breach/reclaim, wick boundaries, zero range, high-volume rejection, strong-demand-at-support composition, semantics labels and future invariance.
- **Acceptance / DoD:** Strong/Weak Demand, Upthrust/Failed Breakout, Spring and support/resistance composites work as transparent technical signals; no “institutional distribution” claim.
- **Risk:** semantic overclaiming and duplicated feature computation.
- **Migration:** pure composites and reason codes transfer directly.

### Phase 4 — Multi-ticker/multi-phase backtest and exact evaluation

#### `P4-BATCH-01` — Compute-once phase runner

- **Goal:** run one strategy/config across arbitrary symbols and phase ranges with independent capital.
- **Requirements:** FR-CORE-008/009/014, NFR-PERF-001/002, TEST-PERF-001, TEST-BT-005.
- **Dependencies:** accepted P2 kernel/profile and basic signal-to-execution binding; P1/P3 signal arrays; explicit timeframe/adjustment data query; phase-end policy frozen before batch benchmark acceptance.
- **Expected modules/files:** batch runner in the backtest domain, request/result schemas, `backtest_service.py` orchestration and API extension.
- **Database impact:** reuse `strategy_lab_runs` JSON artifacts initially; no per-phase session writes during calculation.
- **API impact:** versioned batch request with `symbols[]`, `phases[]`, capital per symbol and execution profile; progress/error results identify symbol/phase.
- **UI impact:** phase editor and per-phase tabs may be delivered with P4-MET-02.
- **Tests:** duplicate/overlapping/invalid phase validation, warmup before earliest phase, one data/feature/signal compute per symbol, phase boundary, open/unsellable position and single-symbol parity.
- **Acceptance / DoD:** identical phase output whether run alone or in batch; no shared cash; instrumentation proves compute-once/slice-many.
- **Risk:** memory growth and partial failures; bound concurrency and return explicit partial status.
- **Migration:** batch orchestration can move to Doraemon jobs; core runner remains portable.

#### `P4-MET-02` — Exact nine metrics, ledger drill-down and export

- **Goal:** produce the Master benchmark table from closed trades without changing calculation signs/`N/A` semantics.
- **Requirements:** FR-CORE-010, TEST-MET-001/002, Master sections 22/I.
- **Dependencies:** P4-BATCH-01 and authoritative P2 ledger.
- **Expected modules/files:** dedicated benchmark metrics module; API result schemas; frontend phase table/drill-down/export adapters; tests.
- **Database impact:** store raw result/ledger and resolved metadata in existing run artifact; normalize later only if querying demands it.
- **API impact:** raw numeric fields plus explicit null/status/unit; presentation formatting excluded.
- **UI impact:** exact columns, stable sort, phase header/range, warnings, trade reconciliation and CSV export.
- **Tests:** zero trades, all winners, all losers, break-even counts as winner, negative loser average, bars held, fees/tax, open positions excluded, renderer-only rounding, export/API parity.
- **Acceptance / DoD:** all nine values reconcile exactly to ledger and capital; no null becomes zero or dash without status semantics; UAT evidence retained.
- **Risk:** conflict with legacy analytics winner convention (`>0`) and rounded intermediates. Keep legacy analytics version separate.
- **Migration:** metrics module is pure; Mizuhara only formats returned numeric/status fields.

### Phase 5 — Symbol Money Flow BB `OHLCV_PROXY`

#### `P5-DATA-01` — Market Data Port and BB contracts

- **Goal:** make quantitative domains consume canonical observations rather than SQLite ORM rows or vendor payloads.
- **Requirements:** FR-CORE-001/012/014, NFR-DQ-001, DATA-CANDLE-001/002/003, TEST-BB-001/002.
- **Dependencies:** verified capability matrix and BB v3; decision on analysis/execution price-stream policy.
- **Expected modules/files:** a new provider-neutral port/DTO boundary plus BB domain models/enums; Sumi candle and future Doraemon adapters; CSV/fixture active-flow adapter for tests. Exact package paths are chosen in the batch ExecPlan.
- **Database impact:** none. Reuse `candles`; do not create a duplicate `ohlcv_daily` table.
- **API impact:** capability and lineage DTOs are internal until P5-BB-02.
- **UI impact:** none.
- **Tests:** canonical unit/date/source mapping, strict missing-vs-zero, incompatible method rejection, source/value/method version hashes, adapter contract and no vendor imports in BB engine.
- **Acceptance / DoD:** BB calculator accepts canonical bars/flow observations from fixture, Sumi and Doraemon adapters without code change; unsupported fields remain unsupported.
- **Risk:** premature “universal” provider abstraction. Implement only methods needed by audited daily proxy and fixtures.
- **Migration:** Doraemon later implements the same port from canonical production tables.

#### `P5-BB-02` — Historical Symbol Technical Flow BB

- **Goal:** calculate BB03/05/10/20/50/200 with the exact v3 OHLCV proxy formula.
- **Requirements:** FR-CORE-012, BBI-SIG-001/003, TEST-BB-001/002 and applicable BB AT01/05–08/13/15/16/18/20.
- **Dependencies:** P5-DATA-01; causal harness; frozen value-source precedence and adjustment policy.
- **Expected modules/files:** pure BB proxy calculator, symbol service/API schemas, focused validation fixtures and a minimal chart adapter.
- **Database impact:** calculate on demand; optional columnar cache keyed by data/method hash, not a mandatory `bb_symbol_daily` table.
- **API impact:** symbol request includes `as_of`, horizons and accepted methods; output includes `flow_method=OHLCV_PROXY`, `value_source`, source ID, method version, quality, warmup, numerator/denominator/activity and coverage.
- **UI impact:** label “Technical Flow BB”; method/value-source/quality badges; default lines 03/05/20/50/200 with optional 10.
- **Tests:** pressure clamp, zero-range, denominator zero→null, rolling identity, T05 day 6 uses days 2–6, [0,100] bounds, prefix invariance, actual-vs-estimated segment, missing value not silently imputed and no anonymous splice.
- **Acceptance / DoD:** reproducible real-data Symbol proxy works without True Flow language; every point is traceable; future rows cannot affect past results; browser evidence verifies labels and boundaries.
- **Risk:** sparse Doraemon actual value and incomplete corporate actions. Degrade/segment instead of mixing or pretending completeness.
- **Migration:** calculator/DTOs move to Doraemon; Mizuhara consumes unchanged method-aware payload.

### Phase 6 — Money Flow measurement validation

#### `P6-MEASURE-01` — BB behavior study, not strategy optimization

- **Goal:** establish what ProxyBB lines measure before freezing events or business labels.
- **Requirements:** BB v3 validation plan; BBI-SIG-002; NFR-DET-001.
- **Dependencies:** P5-BB-02 on real audited daily data; a versioned sample universe and date range.
- **Expected modules/files:** reproducible research runner and machine-readable CSV/JSON outputs; static plots/reports as artifacts, not production feature code.
- **Database impact:** none; immutable result artifacts identify data hash and config.
- **API impact:** none required; may reuse BB calculation service.
- **UI impact:** optional validation view only after report outputs stabilize.
- **Tests:** reproducibility, segmentation completeness, percentile/count reconciliation, lead/lag alignment and no future data.
- **Acceptance / DoD:** required distributions, percentiles, zone time, daily deltas, smoothness/persistence, turn/confluence frequency and cross-horizon lead/lag are delivered by horizon and by method/value-source/quality segment; no P&L used to choose thresholds.
- **Risk:** survivorship and value-source bias. Results must state sample construction and coverage.
- **Migration:** validation datasets become acceptance/golden evidence when domain moves to Doraemon.

### Phase 7 — SUMI-420, Market BB and Breadth gates

#### `P7-UNI-01` — Versioned SUMI-420 framework and candidate v1

- **Goal:** define a portable managed universe and prevent retrospective membership from masquerading as PIT truth.
- **Requirements:** Master G.4/section 26; BB v3 SUMI-420 coverage/version rules.
- **Dependencies:** approved selection purpose/rules and dated source evidence; may proceed on fixtures before approval.
- **Expected modules/files:** new pure universe models/resolver and an immutable versioned YAML/CSV membership artifact; validation report/tests. Avoid database tables in Sumi unless editing/query needs prove necessary.
- **Database impact:** none initially. Doraemon is the future canonical store for membership episodes.
- **API impact:** read-only universe version/coverage contract; responses include mode (`POINT_IN_TIME` or `RETROSPECTIVE_FIXED`), effective dates and source.
- **UI impact:** visible universe version, date applicability, coverage and survivor caveat.
- **Tests:** effective boundaries, no overlapping versions, duplicates, unknown symbols, listed-after/delisted-before cases, stable hash and missing-member coverage.
- **Acceptance / DoD:** `SUMI420_v1_candidate` can be generated from an audited as-of list, but it is promoted to `SUMI420_v1` only after membership/selection/effective date approval. It is never backdated without evidence.
- **Risk:** selection bias hidden by the “420” name.
- **Migration:** immutable artifact maps cleanly to Doraemon universe tables; Mizuhara receives version metadata only.

#### `P7-MKT-02` — Market BB/Breadth framework and publication gate

- **Goal:** implement aggregate-before-ratio and independent breadth on fixtures, then enable real output only for valid universe/date coverage.
- **Requirements:** BB v3 08; BBI-SIG-001/003; applicable BB AT09–12/17/19.
- **Dependencies:** P5 calculator; P7-UNI-01; comparable per-symbol methods and approved coverage thresholds.
- **Expected modules/files:** pure market aggregator/breadth module, schemas, fixture tests and gated service/API.
- **Database impact:** no `bb_market_daily`/`bb_breadth_daily` tables in Sumi MVP; compute/cache artifacts. Doraemon may materialize later.
- **API impact:** result includes universe version/mode, aggregate measures, method, symbol/value coverage, excluded members and quality; returns explicit unavailable/degraded state when gate fails.
- **UI impact:** separate Market BB and Breadth panels; never display VNINDEX volume as Market BB.
- **Tests:** aggregate identity, constructed proof that mean(Symbol BB) differs, value conversion for active-volume method, positive/negative/neutral count sums, threshold version, PIT boundary and coverage failure.
- **Acceptance / DoD:** fixture framework passes; canonical publishing remains disabled unless all dated universe/method/coverage gates pass. Retrospective output is unmistakably labeled research-only.
- **Risk:** cross-symbol method mixing and partial-universe bias.
- **Migration:** aggregator is portable; Doraemon supplies membership/observations and may materialize results.

### Phase 8 — Remaining V1 signals and integration

#### `P8-ICHI-01` — Causal Ichimoku signal semantics

- **Goal:** build bullish/bearish score/reasons while separating visible cloud and projected data.
- **Requirements:** SIG-ICHI-001, TEST-ICHI-001, TEST-CAUSAL-001.
- **Dependencies:** P1 registry; current IndicatorEngine and recent Ichimoku adapter mapping.
- **Expected modules/files:** Ichimoku signal module, projection-aware adapter/API changes, registry/UI tests.
- **Database impact:** none.
- **API impact:** projected points carry explicit projection metadata and are not replay-observed rows.
- **UI impact:** score factors/reasons and projection label.
- **Tests:** displacement fixtures, every-prefix invariance, availability, visible-cloud comparison and no future-candle serialization.
- **Acceptance / DoD:** no historical signal changes when future bars append; projected cloud cannot be mistaken for observed market data.
- **Risk:** pandas-ta extended indexes and Chikou alignment.
- **Migration:** deterministic raw-span/visible-span contract moves with signal domain.

#### `P8-DIV-02` — Confirmed pivots and four-way divergence

- **Goal:** emit regular/hidden bullish/bearish divergence for RSI, MACD histogram and Stochastic only when confirmable.
- **Requirements:** SIG-DIV-001/002, TEST-CAUSAL-002.
- **Dependencies:** P0 harness, P1 registry, indicator outputs and optional P3 trend context.
- **Expected modules/files:** pivot/divergence signal modules, registry/API/UI additions and golden fixtures.
- **Database impact:** none; output includes `pivot_at` and `confirmed_at`.
- **API impact:** delayed availability and paired-pivot evidence are explicit.
- **UI impact:** plot pivot location separately from signal confirmation marker.
- **Tests:** all four definitions, separation bounds, trend context, scale normalization version, equal pivots, NaNs and never-backdated assertions.
- **Acceptance / DoD:** appending future data cannot create a signal before its recorded confirmation date; reasons identify both pivots and oscillator values.
- **Risk:** visual backdating. UI must anchor decision availability at confirmation.
- **Migration:** pure pivot pairing is portable and fixture-backed.

#### `P8-COMP-03` — Technical Health, BB events and strategy namespace integration

- **Goal:** finish composable V1 signals and extend the P1/P2 strategy binding with BB method and minimum-quality constraints. Basic Price/Volume signal resolution must already work before P4; it does not wait for this phase.
- **Requirements:** SIG-HEALTH-001, BBI-SIG-001/002/003, FR-CORE-004/005/006/007, TEST-DSL-001.
- **Dependencies:** P3 signals, P5 BB and P6 measurement decisions for any promoted flow event.
- **Expected modules/files:** health/composite/flow-event modules; strategy schema/adapter/evaluator changes; registry and security tests.
- **Database impact:** persisted strategy/run config stores concrete resolved versions/parameters/method constraints in existing artifacts.
- **API impact:** strategy validation rejects unavailable identifiers, incompatible output types and disallowed flow methods/quality.
- **UI impact:** strategy builder selects registered identifiers and shows dependencies/warnings.
- **Tests:** no double counting, missing BB neutral/absent semantics, at-most-selected confirmations, method/quality guard, arbitrary call/import/attribute escape rejection and old strategy compatibility.
- **Acceptance / DoD:** Signal Engine owns interpretation, DSL only composes, backtest consumes precomputed booleans, and unvalidated research thresholds cannot appear as frozen defaults.
- **Risk:** expanding AST instead of precomputing state. New operators require separate acceptance.
- **Migration:** strategy payload and signal identifiers remain stable across Doraemon/Mizuhara.

### Phase 9 — Minimum Sumi validation workflows

#### `P9-UI-01` — Unified research validation surface

- **Goal:** make the implemented capabilities usable without recreating Mizuhara.
- **Requirements:** UI-SIG-001/002/003, UI-EXP-001, Master Appendix F and FR-CORE-006/010/012.
- **Dependencies:** frozen registry, batch, metrics and BB APIs.
- **Expected modules/files:** incremental existing page/components/API-client changes; prefer shared registry-driven controls over another frontend architecture. Preserve current user-owned Strategy Lab work.
- **Database impact:** none beyond existing run history.
- **API impact:** no new semantics; UI consumes stable contracts.
- **UI impact:** signal catalog, active explanation/date inspector, strategy builder, multi-phase table/drill-down/export, Technical Flow BB lines, measurement charts and quality/warning panels.
- **Tests:** component accessibility/state, backend reason parity, phase/result rendering, `N/A`/negative signs, method/value-source badges, no future rows, console/network errors and 1440×1000 UAT screenshots.
- **Acceptance / DoD:** a user can configure → inspect → compose → backtest → inspect results/BB validation with all limitations visible.
- **Risk:** scope creep and misleading P&L recommendations. Existing ratings remain legacy UX unless separately reviewed; they do not freeze signal/BB semantics.
- **Migration:** components are disposable; API types and UX findings inform Mizuhara.

### Phase 10 — Hardening, performance and migration readiness

#### `P10-PERF-01` — Profile and cache verified hotspots

- **Goal:** meet interactive local research performance without compromising determinism.
- **Requirements:** NFR-PERF-001/002, TEST-PERF-001, TEST-REPRO-001.
- **Dependencies:** stable feature/signal/batch semantics.
- **Expected modules/files:** benchmark harness, shared dependency graph/cache adapter and evidence. Numba only for a measured hotspot.
- **Database impact:** optional columnar cache metadata/files; no semantic source of truth.
- **API impact:** progress/cancellation and timing metadata only; no calculation-semantic change.
- **UI impact:** expose progress/cancellation and timing evidence without adding a new workflow.
- **Tests:** cold/warm 30–50 symbols × 3 phases, invalidation on data/indicator/signal/BB/rule/version change, memory bounds, cancellation and deterministic parity with cache disabled.
- **Acceptance / DoD:** benchmark recorded on reference machine; feature computation once per symbol; cache cannot return stale semantics.
- **Risk:** premature optimization and incomplete cache keys.
- **Migration:** cache is replaceable; keys describe portable domain/data versions.

#### `P10-SEAL-02` — Full correctness/evidence and migration package

- **Goal:** close acceptance traceability and package validated logic for later Doraemon/Mizuhara adoption.
- **Requirements:** all implemented IDs, NFR-OBS-001, TEST-REPRO-001 and repository Definition of Done.
- **Dependencies:** completed scoped V1 phases; data-blocked capabilities may remain explicitly unavailable.
- **Expected modules/files:** acceptance matrix updates, API schema snapshots, golden fixtures, migration notes and retained test/UAT artifacts—not a second program plan.
- **Database impact:** migrations only if earlier authorized batches proved the need; fresh/temp upgrade and rollback verified.
- **API impact:** compatibility and deprecation review; freeze adopted versioned schemas.
- **UI impact:** compatibility and acceptance review of implemented validation workflows.
- **Tests:** complete verification/UAT, clean-install migration, reproducibility, security/secret scan, data integrity/hash and failure-path evidence.
- **Acceptance / DoD:** no P0/P1 failure in implemented scope; every claim links to tests/browser evidence; exclusions/blockers are visible; reviewer signs off.
- **Risk:** declaring completion while Market BB/True Flow remain unavailable. Capability badges and acceptance matrix prevent that.
- **Migration:** produce a subsystem-by-subsystem extraction map, fixture suite and versioned contracts for Doraemon/Mizuhara.

## G. Parallel Data R&D Plan

Data R&D begins after implementation baseline capture and runs independently of Price/Volume work. No collector starts merely because an endpoint exists: semantics, access, license/persistence and a bounded retention plan must first be approved. Categories below use the required evidence boundary.

| Candidate / category | Question | Minimum test | Credential/trial | Expected data | Success criterion | Failure criterion | License concern | Blocks MVP? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **KBS — `VERIFIED_CURRENT_CAPABILITY`** | Can session ticks/quotes validate a research classifier, and what does `LC` mean? | With authorization, capture small FPT/SHS/BSR overlap across sessions; measure duplicate IDs, clock/quote alignment, unknown rate and reconcile totals | No provider secret currently required; retention authorization still required | Current trades, `LC`, accumulated totals, 3-level board, matched-by-price buckets | Written semantics or high-quality labeled agreement; stable capture/reconciliation and permitted persistence | Side meaning remains undocumented; gaps/duplicates/clock mismatch or no retention right | Public reachability is not permission to bulk persist/redistribute | No; only realtime research/forward-history option |
| **SSI FastConnect — `RESEARCH_CANDIDATE`** | Can official daily fields improve proxy value/PIT data, and can X-TRADE provide validated forward classified flow? | Authenticated daily sample across HOSE/HNX/UPCoM/index plus one live session; test ALL/SUMI-420 subscription, reconnect/gaps, BU/SD/unknown and EOD reconciliation | Consumer ID/secret and service agreement required; none configured in audited projects | Daily matched/put-through/foreign value, security metadata, live trade Side and quotes | Stable access, exact semantics, sufficient coverage/history, documented persistence/non-display rights; side validation meets threshold | Missing historical trades, unacceptable unknown/gaps, ambiguous Side, restrictive rights or unstable access | Fees, rate limits, internal persistence and derived-output redistribution | No for core/Proxy; yes only for SSI-based forward classified mode |
| **vnstock_data order flow — `RESEARCH_CANDIDATE`** | Does `order_flow_history()` actually provide the v3 canonical active buy/sell fields over useful history? | Install/use only in an isolated authorized spike; query ACB/FPT/HPG/VCB, one HNX and one UPCoM for latest, one month and one year; record upstream/source, rows, units, missing/unknown | Sponsor/extended entitlement likely required | Active buy/sell value and volume, unknown, historical daily order flow | Multi-year, batchable SUMI-420 coverage with documented executed/classified semantics and persistence rights | Fields are order intent, shallow/inconsistent history, unknown upstream, access instability or rights prohibit use | Software license does not grant third-party data rights | No; potentially accelerates True/Active history |
| **FireAnt `_AC` / export — `RESEARCH_CANDIDATE`** | Is documented daily Active Buy/Sell volume accessible historically and automatable without scraping? | Use approved account/export/API to sample same six instruments/dates; compare `_AC` values to independent session totals | Existing Doraemon FireAnt token is for financial providers and must not be assumed sufficient; product entitlement may be needed | Daily active buy/sell volume/value or export series | Semantics and units documented; useful depth/coverage; export/API automation and persistence allowed | UI-only, shallow, unbatchable, semantic mismatch or prohibited retention | Account terms, automation/export and derived-data rights | No; historical active-flow option only |
| **FiinQuant — `FUTURE_PROCUREMENT_OPTION`** | Does the commercial feed provide historical tick/order-book depth or labeled side, not just realtime marketing claims? | Vendor trial and schema walkthrough; six-symbol historical/realtime sample; clock/sequence/drop recovery and correction test | Commercial trial/quote | Tick trades, order book, possibly smart-money and historical merge | Contract proves fields/depth/exchanges/SLA/persistence; sample supports Tier B/C or active daily aggregation | No historical tick/book, undefined side, cost/rights exceed value | Non-display, storage, derived analytics and redistribution | No; candidate production quality upgrade |
| **HOSE/HNX official feeds — `FUTURE_PROCUREMENT_OPTION`** | What exact trade/quote/archive/lifecycle products and non-display rights are available? | Written product dictionaries, sample files/messages and quotes; validate side/session/put-through/corrections and history | Direct contracts and budget | Authoritative exchange trade/quote/EOD/lifecycle data | Required fields and rights proven for the relevant exchange at acceptable total cost/effort | No aggressor/classification field, insufficient archive, prohibitive license/cost | Material connection, end-user/non-display and redistribution restrictions | No for Sumi MVP; may block highest-integrity production scope |
| **SUMI-420 membership — `RESEARCH_CANDIDATE`** | Which approximately 420 securities belong to v1 and from what effective date is that membership defensible? | Start `SUMI420_v1_candidate` from Doraemon's audited active list as of 2026-09-11; apply documented eligibility/liquidity/security-type rules; compare with exchange status and data completeness | No secret; needs product owner selection policy and authoritative lifecycle/status evidence | Symbol, exchange, security type, inclusion reason, evidence date, effective interval | Approved immutable list/version/hash, no ineligible securities, coverage gate satisfied from effective date | Arbitrary survivor selection, missing exchange/status, or insufficient bar/flow coverage | Source attribution/redistribution for membership data | No for Symbol BB; yes for canonical Market BB/Breadth |

Priority: membership and vnstock_data/SSI evidence first; KBS overlap only after retention approval; FireAnt next; FiinQuant and official exchange procurement when lower-cost paths fail or production quality justifies them. API/export precedes RPA. A useful realtime source should begin forward collection promptly **after** semantics and rights pass—not before.

## H. Money Flow Validation Plan

### H.1 Measurement validation — mandatory first

Use real historical Symbol `OHLCV_PROXY` only after P5. Freeze dataset hash, method version, value-source policy, horizons and sample construction. Produce machine-readable tables plus charts for:

- distribution and percentiles for BB03/05/10/20/50/200;
- time below candidate 20/30 and above candidate 70/80, always labeled research references;
- daily delta distribution, rolling volatility/smoothness and autocorrelation/persistence;
- denominator-zero, warmup and missingness counts;
- candidate TURN_UP/TURN_DOWN counts by level and minimum separation;
- confluence counts across declared horizon sets and tolerance grid;
- cross-horizon lead/lag statistics with exact alignment convention;
- line charts with event markers and source/method/value-source/quality boundaries;
- results segmented by provider, `flow_method`, `value_source`, quality band, exchange, liquidity bucket and date regime;
- coverage comparisons where actual matched value overlaps estimated value; never silently pool the segments.

Measurement acceptance requires deterministic reruns, bounded [0,100] values, reconciled counts, no future leakage, explicit caveats and a written recommendation for which events remain experimental. It expressly excludes strategy P&L, Net Profit, Sharpe or parameter selection by return.

### H.2 Predictive/backtest validation — only after measurement acceptance

After thresholds/events are versioned as hypotheses, evaluate forward returns and strategy contribution with temporal holdout/walk-forward splits, sensitivity ranges and baseline comparisons. Separate Price/Volume-only from Price/Volume+BB strategies and require method/quality constraints. Report sample size, turnover, costs, stability and negative results. A threshold is not promoted because it maximizes one historical phase; promotion requires stability across symbols/phases and a documented product interpretation.

If future active/classified flow becomes available, compare it with overlapping ProxyBB as **two methods**: agreement, rank correlation, event precision/recall, coverage and lead/lag. Never use overlap to relabel older proxy history as True Flow.

## I. Test Strategy and Phase Traceability

| Test family | Core assertions | Phase(s) |
| --- | --- | --- |
| Formula/boundary | Candle features, patterns, RVOL, VSA, regimes and BB exact identities at equality/zero-range/zero-denominator edges | P1, P3, P5 |
| Determinism/reproducibility | Same data/config/versions/hash → identical features, signals, ledger, metrics and artifacts | P0–P10 |
| Future invariance | Full-series prefix equals truncated input at every observable date; future mutations cannot alter past | P0, P1, P3, P5, P8 |
| Warmup/NaN/DQ | No premature claim; missing ≠ zero/false; partial quality explicit; no inf/NaN leakage through JSON | P1, P3, P5 |
| Pivot confirmation | `pivot_at` separate from `confirmed_at`; divergence emitted only at latest confirmation | P8 |
| Ichimoku displacement | Visible/projection indexes separated; no future candle enters score | P8 |
| AST safety | Calls, imports, attributes, arithmetic/unknown identifiers rejected; registered signal types validated | P1, P8 |
| Execution timing | Close[T] signal cannot fill Close[T]; next valid event/open, final-bar and pending-order behavior | P2 |
| Settlement/calendar | Trading-session T+, holidays, locked limits, lot/tick/band, stop/trailing precedence and phase-end sellability | P2, P4 |
| Phase boundaries | Warmup outside phase, no trade leakage between independent phases, open positions explicit | P4 |
| Batch parity/performance | Single vs batch equality; compute called once per symbol; independent capital | P4, P10 |
| Exact metrics | Nine columns, closed trades only, break-even winner, negative losses, `N/A`, renderer-only rounding | P4 |
| BB bounds/rolling identity | 0/50/100, [0,100], rolling session windows, T05 day 6 uses days 2–6 | P5 |
| Method boundary/lineage | Proxy never True; source/value/method changes create explicit segments; allowed-method strategy guard | P5, P8 |
| SUMI-420 membership | Version effective intervals, hash, duplicate/ineligible/missing members, retrospective label | P7 |
| Market aggregation/breadth | Aggregate measures before ratio; never average scores; breadth counts and coverage reconcile | P7 |
| API/replay safety | `as_of`/current-index cap, strict schemas, vendor fields absent, raw numeric metric/status preservation | P1, P4, P5, P7 |
| Browser evidence | New visible behavior, method badges, reasons, phase results, no console/page errors at 1440×1000 | Every UI batch |

BB phases must reference the applicable v3 acceptance tests rather than copying divergent formulas. No test may be weakened to accommodate implementation behavior.

## J. Data / Database Plan

### Persistence decisions

| v3 concept / current entity | Sumi MVP decision | Later / owner | Reason |
| --- | --- | --- | --- |
| `data_source` | **Domain/config metadata now; no table** | Doraemon canonical source registry | Few research sources; source-controlled definitions and capability manifest are enough |
| `active_trade_raw` | **Not now** | Doraemon when an authorized forward collector passes gates | No accepted source; raw tick reliability/retention is production data work |
| `active_flow_daily` | **Fixture/CSV DTO only now** | Doraemon canonical table after source acceptance | Enables formula/integration tests without fake production capability |
| `ohlcv_daily` | **Do not create** | Use Sumi `candles` through Market Data Port; Doraemon already has daily table | Avoid duplicate truth and migration cost |
| `bb_symbol_daily` | **Compute on demand; optional hash-keyed columnar cache** | Doraemon may materialize production results | Research iteration benefits from recalculation/versioning; cache is disposable |
| `bb_market_daily` | **Fixture/on-demand only after gate** | Doraemon may materialize validated production output | Real publication currently data-blocked |
| `bb_breadth_daily` | **Fixture/on-demand only after gate** | Doraemon later | Same as Market BB; threshold version is research state |
| `sumi_universe_version/member` | **Immutable versioned YAML/CSV artifact initially** | Doraemon membership/episode tables | Approximately 420 rows do not justify a Sumi migration; source control gives review/hash/history |
| Signal definitions/results | **Definitions in code; results transient/cache** | Doraemon may cache/materialize selected outputs | Master E.8 discourages persisting every boolean |
| Backtest run metadata/ledger/results | **Pure in-memory run then store completed JSON in existing `strategy_lab_runs` where size permits** | Normalize in Doraemon or later Sumi migration only if query/volume needs prove it | Avoid practice-session side effects and premature schema |
| Market rules/calendar | **Versioned immutable resources** | Doraemon authoritative service/store | Reproducible and portable; no runtime editing requirement yet |

### Mandatory data rules

- Every query pins symbol, timeframe, adjustment mode, source/priority and `as_of`; duplicate/mixed streams are rejected or explicitly resolved.
- Analysis and execution price streams are distinct contracts. Corporate-action uncertainty degrades or excludes affected runs.
- Missing session, zero volume and source outage are different states.
- Cached/calculated artifacts include data hash, source, adjustment, indicator/signal/BB/rule/execution versions and phase definitions.
- No implementation/test writes `backend/sumi.db`; migrations, if ever approved, run against temporary copies/databases with upgrade/rollback evidence.

## K. API Plan

Target routes are candidate public contracts, not claims that these routes currently exist. Exact prefixes follow existing FastAPI composition during each batch.

| Contract | Request essentials | Response essentials / invariants |
| --- | --- | --- |
| Signal registry | optional category/status | definitions, versions, params schema/defaults, dependencies, warmup/delay, reason schema |
| Signal calculation | symbols/session, timeframe, adjustment, start/end or `as_of`, requested definitions with concrete params | values/states/scores, reason codes/intermediates, version/hash, quality, `available_at`; never future rows |
| Strategy validation | versioned entry/exit AST over registered identifiers plus resolved signal configs | compiled identifiers/types, dependencies/warmup, errors and method/quality requirements; no executable code |
| Batch backtest | strategy version/config, symbols, phases, capital per symbol, execution/fee/rule profiles, data snapshot | run metadata, phase rows with exact nine metrics, ledger links, warnings, DQ and partial-failure status |
| Symbol BB | symbol, date range/`as_of`, horizons, allowed methods and minimum quality | method-aware points with value source, numerator/denominator/activity, coverage, warmup, quality and versions |
| Universe | universe ID/version/date | members, evidence/effective interval, mode, hash and coverage |
| Market BB/Breadth | universe version/date range/horizons/method constraints | aggregate primitives/BB plus separate breadth and coverage; explicit unavailable state on gate failure |
| Capability manifest | source-neutral requested feature family | `VERIFIED_CURRENT_CAPABILITY`, `RESEARCH_CANDIDATE` or `FUTURE_PROCUREMENT_OPTION`; supported dates/fields/quality, no credentials/vendor payload |

API rules:

- Vendor response keys never escape adapter/normalization boundaries.
- Raw numeric metrics are not formatted; null includes a reason/status.
- `methodology_version`, `flow_method`, `value_source`, source identity, quality and coverage are mandatory BB lineage.
- Legacy routes remain until a versioned compatibility/deprecation test says otherwise.
- Sumi never proxies secrets to the browser. Doraemon access remains server-side.

## L. Sumi UI Plan

Use existing Replay, Backtest and Strategy Lab surfaces where practical.

1. **Signal Catalog:** category filter, Vietnamese label/short meaning, version/status, 2–5 ordinary parameters, presets and advanced disclosure.
2. **Explanation inspector:** selected symbol/date, state/score, reason codes rendered with actual intermediates, warmup/quality and `available_at`.
3. **Strategy builder:** registered fields/operators only; resolved parameters/version visible; BB method/quality constraints when used.
4. **Batch backtest:** ticker multi-select, phase editor, capital and named profiles, progress, phase tables, trade drill-down, export and assumptions/warnings.
5. **Technical Flow BB:** six supported horizons, default five visible, method/value-source/quality badges, explicit proxy language and method-boundary markers.
6. **Measurement validation:** distribution/percentile tables, time-in-zone, turns/confluence, lead/lag and segmented line charts. No “best threshold by profit” control.
7. **Market/Breadth:** hidden/unavailable until gated; when enabled, universe version/mode/coverage is prominent and panels stay separate.

Do not introduce a new design system, advanced visual builder, vendor setup UI, collector dashboard or Mizuhara-scale navigation in Sumi. Current P&L-derived Strategy Lab ratings are not authoritative signal/BB explanations and should be reviewed in the relevant UI batch rather than expanded.

## M. Migration Plan

| Subsystem | Sumi prototype ownership | Future Doraemon ownership | Mizuhara consumption | Difficulty | Coupling to avoid now |
| --- | --- | --- | --- | --- | --- |
| Market Data Port | Interface + Sumi/Doraemon read adapters | Canonical observations/capability manifest | Stable DTOs only | Medium | SQLite ORM/vendor JSON in domain |
| Indicator calculation | Retained backend engine and parity fixtures | Adopt/host validated calculations or compatible service | Indicator API | Medium | pandas-ta column names in UI/product semantics |
| Signal Engine/Registry | Pure definitions/registry/calculator | Production compute/registry | Registry/results/reasons | Low–Medium | FastAPI, ORM, React labels inside formulas |
| Money Flow BB | Pure formula/method/quality contracts and validation | Data normalization, calculation/materialization and collectors | Method-aware series/events | Medium | Direct KBS/SSI/FireAnt imports; duplicate BB formulas |
| SUMI-420 | Version artifact/resolver | Canonical membership episodes/evidence | Universe/version/coverage | Low | Backdated current list and UI-owned membership truth |
| Strategy DSL | Existing safe evaluator plus versioned signal namespaces | Production validation/execution orchestration | Builder payloads | Low | Arbitrary functions or Python expressions |
| Backtest kernel | Pure daily execution/ledger/reference fixtures | Production job/API runtime if adopted | Batch results/drill-down | Medium | ReplaySession/TradeLifecycleService side effects |
| Market rules | Versioned resolver/resources | Authoritative calendar/rules | Named assumptions/warnings | Medium–High | Scattered constants/calendar arithmetic |
| Metrics | Pure exact aggregator | Production reporting service | Raw numeric/status output | Low | UI rounding and legacy analytics conventions |
| Sumi UI | Research validation only | None | UX findings/types migrate; components may be replaced | Intentionally low | New design system/vendor-specific controls |

Migration readiness means the same fixture input produces equivalent domain output after extraction. It does not require sharing database models or frontend code.

## N. Critical Path

```text
baseline + causal harness
        ↓
Signal contracts + Volume Spike
        ├──────────────→ Core Price/Volume signals ──────┐
        │                                                │
        └→ Pure next-event execution → market rules ─────┤
                                                         ↓
                                 multi-ticker/multi-phase + exact metrics

Market Data Port → Symbol OHLCV_PROXY → measurement validation
                                              ↓
SUMI-420 approved membership/data gate → Market BB/Breadth publication
```

Sequential necessities:

- Signal contract precedes signal catalog and strategy signal identifiers.
- Causal kernel precedes connecting new signals to automated backtests.
- Phase 1 validates signal values, reasons, availability and isolated DSL resolution only. Existing Strategy Lab/backtest P&L, rankings and recommendations are not admissible evidence for signal quality. Phase 2 must pass execution/market-rule/ledger gates before execution results are used, and Phase 4 must pass exact-metric gates before benchmark tables are trusted. These gates do not establish predictive validity.
- Pure ledger precedes exact metrics; market rules precede claims of Vietnam execution correctness.
- Market Data Port/method lineage precede BB calculation.
- Symbol proxy precedes measurement; measurement precedes threshold/business-label freeze.
- Approved dated SUMI-420 membership plus coverage precedes canonical Market BB/Breadth.

Safe parallel work:

- Data R&D runs beside all core Price/Volume/backtest phases.
- Signal catalog and causal kernel can be developed in separate sequential batches without waiting for True Flow.
- Symbol BB proxy can proceed without active-flow procurement.
- UI component design may prototype against frozen fixtures, but integration waits for API contracts.

Does **not** block the Sumi MVP: unavailable True Flow, historical ticks/order book, provider procurement, raw collector, full-market PIT reconstruction, Market BB/Breadth, RPA, ML or shared-cash portfolio work.

Capability-specific blockers:

- Production active/classified BB: accepted semantic and licensing evidence.
- Canonical SUMI-420 Market BB/Breadth: approved membership/effective date, comparable per-symbol inputs and coverage policy.
- Full historical Vietnam execution claim: authoritative date-versioned rules/calendar and corporate-action policy.

## O. Recommended First DEV Slice

### `P1-SIG-01 + P1-SIG-02` — Explainable causal Volume Spike

**Scope:** create the minimal signal models/registry/shared candle-volume frame; implement prior-window Relative Volume and Volume Spike; expose registry and replay-scoped calculation; add a small configuration/explanation UI. Do not connect the signal to automated backtest execution, create BB code/tables, add migrations, or change provider behavior.

Include isolated DSL resolution of precomputed signal fixtures through the binding adapter specified in P.4; production strategy configuration/execution integration comes in Phase 2. Phase 0 baseline capture and causal harness are prerequisites, not optional work implied to be complete. Phase 1 is one vertical batch with reviewed milestones; it is not permission to continue automatically into Phase 2.

**Expected files/modules:** a small new signal-domain package (exact filenames confirmed in its ExecPlan), signal schema/service/router, focused backend tests, frontend signal API/types/catalog/explanation components and tests, plus one ExecPlan. Reuse `IndicatorEngine`, `ReplayService` current-index slicing and existing frontend component conventions.

**Requirements:** FR-CORE-001/002/003/004/006/007/011, NFR-DET-001, NFR-DQ-001, NFR-MAINT-001, NFR-UX-001, SIG-VOL-001/002, UI-SIG-001/002/003, UI-EXP-001, TEST-SIG-001/002/003, TEST-DSL-001 and TEST-CAUSAL-001.

**Tests:** exact prior-N exclusion, equality threshold, zero/missing baseline, warmup/NaN, invalid params, determinism/version/hash, every-prefix invariance, API current-index cap, explanation parity, component tests and focused/full browser UAT at 1440×1000.

**Definition of Done:** a user can configure Volume Spike and see a deterministic reason such as relative volume versus threshold on the currently visible replay bar; no future point is returned; isolated DSL resolution preserves unknown/quality semantics; all mapped tests/full gates pass; artifacts and screenshots are retained; production DB hash is unchanged; reviewer verifies no execution, P&L calibration, BB or unrelated changes.

**Why first:** it delivers one complete user-visible capability, proves the contracts reused by every later signal, relies on audited OHLCV rather than blocked flow data, and stays isolated from the known non-causal automated backtest until Phase 2 fixes it.

### Decisions that genuinely require owner input

| Decision | Required by | Recommended default | Blocking impact |
| --- | --- | --- | --- |
| Implementation baseline for the current dirty checkout | Before any DEV writes | Capture HEAD, index, tracked/untracked contents and classifications under P.2; a commit is optional and requires separate authorization | Immediate baseline prerequisite; no blanket staging or deletion |
| Phase-end policy | Before P4; earlier if P2 exposes a configurable phase-end contract | `force_liquidate_at_phase_end=false`; report open/unsellable positions | Does not block Phase 1 |
| Historical market-rule/calendar and corporate-action sources | Before accepting the relevant P2/P4 historical execution profile | Research sources in parallel; pin effective-date evidence and conservative daily assumptions | Does not block Phase 1; blocks unsupported historical correctness claims |
| SUMI420_v1 selection rule and effective date | Before P7 canonical publication | Candidate from audited membership as of 2026-09-11; approve rules and evidence-backed effective date | Does not block Phase 1–6 or the universe fixture framework |
| Provider trials/retention budget | Before the relevant acquisition trial or collector | SSI/vnstock_data first, KBS retention explicitly scoped, procurement later | Does not block core DEV or historical Symbol ProxyBB |
| Existing Strategy Lab P&L rating language | Before adopting it in the new validated workflow | Preserve legacy UI; prohibit its use as Phase 1/BB measurement evidence | Does not block Phase 1 |

The BB proxy formula, v3 method enum, no-splice rule, 50 balance meaning and measurement-before-P&L order are already locked; they are not repeated as owner questions.

## P. Agent Handoff Review and Execution Controls — 2026-09-12

### P.1 Review verdict and evidence

Research is sufficient to stop broad architecture exploration and prepare bounded DEV batches. The previous A–O roadmap was not a self-contained implementation instruction: it left exact contracts, invalid-input behavior, file choices and verification commands to the implementer. No model, including the user-designated Gemini Flash 3.8, can be promised bug-free execution from a plan. Readiness is established by frozen batch contracts, independent expected results, executable checks and reviewer inspection; no model capability benchmark is asserted here.

| Finding | Evidence / consequence | Resolution |
| --- | --- | --- |
| Phase 1 could be evaluated with biased legacy P&L | `backend/app/services/backtest_service.py` still uses `MARKET_AT_CLOSE` for close-derived signals | Phase 1 is signal validation only; N defines the execution and metrics gates |
| Basic signal binding appeared deferred to P8 although P4 needs it | P4 consumes P1/P3 signals; P8 previously owned strategy namespace integration | Isolated binding in P1; causal runtime integration in P2; P8 adds advanced/BB constraints |
| Dotted domain IDs conflict with existing expression grammar | `rule_evaluator.py` rejects `ast.Attribute`; the Master uses `volume.spike` in registry but `volume__spike` in strategy examples | Freeze explicit ID-to-identifier mapping; keep AST whitelist unchanged |
| Missing values can become a positive negated condition | Existing evaluator returns false for missing numeric comparisons, and `not` negates that; bare `not None` also evaluates true | New signal binding checks availability/quality for all referenced inputs before evaluation; invalid input makes the entire signal-bound rule unavailable |
| Master shorthand DSL examples are not all executable today | `evaluate_rule_dsl` requires object nodes; Master illustrates string items inside `all` | P1 uses explicit `eq` object nodes; no new shorthand parser or AST syntax |
| Replay history does not automatically include pre-session warmup | `ReplayService.get_candles` uses session start/end and `current_index + 1` | First slice explicitly warms up within the visible session prefix; no new hidden prehistory query |
| Replay UI can show stale signal results after rewind/config change | Signals add asynchronous derived state to existing replay navigation | Request identity includes session, index, configuration and version; stale responses are discarded and previous values cleared |
| Some task cards contain many independent capabilities | P3 covers many patterns/regimes; P8 combines several advanced families | Split into bounded deliveries under P.7 before assigning later phases |
| Old completion prose overstates evidence for the new scope | V3 handoff says production ready; current automated timing defect and absent new modules are verified | Earlier UAT/test counts describe earlier scope only; no inherited green status for new acceptance IDs |

Source/data authority remains the Master + latest BB v3 + verified audits. Earlier ADR provider surveys do not override audited access/semantics/rights. BB measurement validation starts after P5 produces a usable dataset; it cannot run empirically immediately alongside Phase 1. Acquisition R&D and historical market-rule/calendar/corporate-action evidence can proceed earlier.

### P.1a Doraemon/provider targeted freshness gate

Before any implementation batch that consumes Doraemon data or relies on a provider capability, DEV must read `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` and `docs/research/data_capability_matrix.csv`. It must then perform a read-only freshness check limited to the exact source/endpoint or table, fields, symbols/universe, and date range required by the batch. A broad Doraemon re-audit is explicitly out of scope.

The batch evidence must identify the source/release when available, observation time, access path, latest available market date, relevant row/field coverage, and semantic/method classification. Record retention or redistribution status when it affects the planned use. If observed schema, freshness, coverage, semantics, access, or rights differ from the audit, update the audit evidence and corresponding matrix row before coding; stop for reviewer adjudication if a mandatory capability regressed or remains unverified. The check must not mutate production data, provider/account state, or token state.

This gate applies to Phase 5 Symbol BB when Doraemon is consumed, Phase 7 market/universe work, provider trials, data acquisition, and any earlier batch newly coupled to Doraemon. It does not apply to a self-contained signal/backend batch such as P1R-01A that uses no Doraemon data or provider capability.

### P.2 Baseline freeze packet — next preparatory task

Review observation, **not a frozen baseline**: HEAD was `89e04fb00210027c8b08d3fd0116b5773ac26f17`; porcelain status showed **40 modified tracked files, 159 staged deletions and 53 untracked files**. These include production replay/trade/UI changes, old document deletions, research documents, and new UI/UAT files. Ownership or intent cannot be inferred from Git status alone. The staged deletions must not be silently committed as part of a new feature. Git also warned that the global ignore file was unreadable; record that limitation when reproducing the inventory.

The next baseline-only assignment must:

1. Record branch, full HEAD, exact staged and unstaged path/status lists, rename pairs, untracked paths, timestamp and relevant tool versions. Read applicable nested `AGENTS.md` files before choosing writable paths.
2. Classify every entry as evidenced intentional project work, research/handoff artifact, or unrelated/pre-existing work. Add `intent_unconfirmed` where evidence does not establish intent; do not manufacture an owner decision or delete unknown files.
3. Retain separate staged/unstaged binary-capable diffs and a per-path content-hash manifest, including untracked source/spec files. Preserve recoverable copies of changed/untracked content in a local ignored baseline artifact directory. Do not copy secrets or production market/trading data into a tracked artifact. Record index contents separately from working-tree contents; HEAD plus a diff statistic is insufficient.
4. Record the production database hash read-only, without connecting/importing the application against it. Include file-presence/hash state of WAL/SHM companions if present; hashing the main file alone is not a consistent database snapshot when a writer is active. Never checkpoint or alter the database for this task.
5. Record allowed first-batch files and overlap with existing changes. A dirty checkout is acceptable if it is reproducibly captured and no concurrent writer changes those paths. If the manifest changes during capture, recapture or report the conflict; do not label it frozen.
6. Record the artifact paths/hash and any unresolved overlapping ownership in this document or the first ExecPlan, then STOP. Freeze does not mean approval of existing changes. Do not stage, commit, reset, stash, clean, delete, create a branch/worktree, start product servers, or implement a feature as part of this packet.

An optional baseline commit is not required for Phase 1 if the recoverable snapshot and incremental review are sound. User clarification is needed only for unresolved ownership that actually overlaps the batch; unrelated preserved changes do not create a blanket blocker. Baseline tests are a separate P0 readiness check: old report counts must not be reused as new test results.

### P.3 Phase 1 frozen scope and milestone order

Implement `P1-SIG-01 + P1-SIG-02` as one bounded vertical batch after P0 baseline/harness readiness. This is the only Phase 1 scope: daily session OHLCV → numeric Relative Volume → boolean Volume Spike → metadata/reasons → isolated DSL resolution → replay configuration/inspection. Only `period` and `multiplier` are user parameters. Optional candle direction, ATR filtering, other signal families, chart markers, persisted signal settings and trade/P&L workflows are excluded from this first batch.

| Milestone | Deliverable | Exit evidence before next milestone |
| --- | --- | --- |
| M0 | ExecPlan with exact baseline, allowed files, contracts and commands | Reviewer checks prerequisites, source IDs and acceptance mapping; unresolved semantic conflict stops the batch |
| M1 | Pure signal models/registry, formula and fixture outputs | P.5 numerical and future-invariance cases pass; no application DB/API imports in formula modules |
| M2 | Binding adapter, registry/API and temporary-DB integration | Name mapping, invalid/negated inputs, replay index cap and schema errors pass |
| M3 | Small replay configuration/explanation panel | Backend values displayed verbatim; loading/error/warmup and stale-response cases pass |
| M4 | Full gates, retained evidence and incremental diff review | Reviewer accepts acceptance map, product UAT/screenshots and baseline preservation; STOP |

Milestones are checkpoints inside one batch, not separate release claims. No partial milestone is called a completed user feature.

### P.4 First-slice contract to copy into its ExecPlan

These choices refine the Master for this bounded slice. A DEV agent must not silently change them. Any contradiction with source requirements goes to the reviewer before dependent coding.

| Contract | Required behavior |
| --- | --- |
| Identity | Registry IDs `volume.relative_volume` (numeric) and `volume.spike` (boolean), version `1.0.0`; AST aliases `volume__relative_volume` and `volume__spike`. Explicit registry mapping, not arbitrary string replacement. One resolved configuration per ID in the first slice; reject duplicate IDs rather than overwrite them. |
| Parameters | `period` strict integer, default 20, range 1–252; `multiplier` finite positive number, default 2.0, maximum 100. Reject bool-as-number, unknown fields, null, NaN/Infinity and invalid versions. Limits are slice resource-validation choices, not calibrated research thresholds. Relative Volume has only `period`. |
| Formula | At zero-based bar t, baseline is arithmetic mean of exactly volumes `[t-period, t)`. RVOL = current volume / baseline; spike = RVOL >= multiplier, without rounding before comparison. No current-bar baseline inclusion, fill-forward or shortened window. |
| Warmup/quality | Before period prior bars exist: null value/state and `INSUFFICIENT_HISTORY`. Non-finite/negative current or window volume: null and `INVALID_VOLUME`. Positive-length all-zero baseline: null and `ZERO_BASELINE`. Current zero on a positive baseline: RVOL 0 and spike false with `VALID`. Zero is a valid prior observation; missing is not zero. Invalid bars do not get dropped from windows. |
| Inputs | Chronologically increasing unique daily bars from the same session symbol/timeframe/adjustment. Reject duplicate/non-monotonic timestamps; do not silently sort/collapse different observations. Support `1D` only initially; unsupported timeframe produces a documented validation error. |
| Session boundary | Read the existing authoritative session prefix using `ReplayService.get_candles`; no client override of symbol/range/index and no post-index data returned. Initial warmup uses session history only, matching the current replay boundary. The panel explains the required prior bars. |
| Availability | Signal for a daily bar is available only at that bar's close event. Preserve existing candle timestamp convention and additionally declare `availability_event=BAR_CLOSE` with the bar index; do not reinterpret a midnight/session date timestamp as a known exchange close instant. |
| Output | Echo session ID, observed index, bar timestamp, resolved params, signal version and deterministic params hash. Each point includes nullable typed value, quality/reason codes, baseline/current volume/RVOL/threshold as applicable and availability metadata. Omit score for these outputs; do not invent a confidence score. No non-finite JSON numbers. |
| Hash | Canonical UTF-8 JSON of explicit signal name/version/resolved params, sorted keys, compact separators, normalized parameter types and no NaN; SHA-256. Defaults omitted by the client resolve to the same hash as explicit defaults. Freeze one expected serialization/hash fixture. |
| DSL boundary | Use existing evaluator on precomputed typed snapshots through a new adapter. AST example: `volume__spike == True`. DSL example: `{"eq":["volume__spike",true]}`. Reject unknown identifiers/calls/attributes and unsupported shorthand. Before evaluation, inspect all referenced signal dependencies (including previous values for crosses): any invalid/unavailable dependency returns nullable result plus reason and does not enter boolean evaluation. `not` cannot convert unknown to true. Existing legacy evaluator semantics remain unchanged. |
| API | Planned new `GET /api/signals/registry` and `POST /api/signals/replay/{session_id}/calculate`. Request contains a bounded nonempty list (maximum two) of explicit signal name/version/params; response is confined to the server-observed prefix. Missing session 404; malformed/unsupported request 422. No generic arbitrary-symbol history endpoint in this slice. |
| UI concurrency | Request key includes session ID, visible index, requested signals, versions and resolved configuration. Clear stale values on change; discard late responses not matching the current key/index, including rewind at the same timestamp/config switch. Never retain the previous symbol's reason while a new request is pending. |
| Persistence and dependencies | No migration, new dependency, network provider access, strategy save format change or writes to practice trades/orders. Formula core consumes input DTOs; FastAPI/ORM remain in service/schema layers. |

Expected **new candidate paths**, to verify absent and freeze in M0: `backend/app/domain/signals/{__init__,models,registry,volume}.py`, `backend/app/domain/strategy/signal_binding.py`, `backend/app/schemas/signal_schema.py`, `backend/app/services/signal_service.py`, `backend/app/api/signals.py`, `backend/app/tests/test_signals.py`, `backend/app/tests/test_signal_binding.py`, `backend/app/tests/test_signals_api.py`, `frontend/src/api/signalsApi.ts`, `frontend/src/types/signals.ts`, `frontend/src/components/signals/SignalInspector.tsx` and its component test. These are planned files, not claims that they exist.

Expected **existing integration points**: `backend/app/main.py` router registration and a minimal composition change in `frontend/src/components/replay/ReplayWorkspace.tsx`; inspect `ReplayWorkspaceController.tsx` only as needed for authoritative session/index state. Keep data fetching/state in a focused component/hook rather than expanding replay composition into a signal engine. Add focused assertions to the existing UAT harnesses. The ExecPlan must list each exact changed file; equivalent modules discovered at M0 supersede candidate names with a recorded reason.

Protected first-slice surfaces: `backtest_service.py`, `trade_lifecycle_service.py`, existing `rule_evaluator.py` syntax/semantics, strategy save schemas, market providers, chart-library adapters, migration files and production DB. A proposed change to these is outside this batch and must return to reviewer. Read-only reuse/import of the safe evaluator is expected.

### P.5 Independent acceptance oracles for Phase 1

Use fixed expected arithmetic below, not production formula code to generate expected results. The shortened `period=3` cases are test fixtures; the production default remains 20.

| Case ID | Input / action | Expected result |
| --- | --- | --- |
| P1-OR-01 | Volumes `[100,200,300,400,600]`, period 3, multiplier 2 | t=0..2 unavailable; t=3 baseline 200, RVOL 2, spike true; t=4 baseline 300, RVOL 2, true |
| P1-OR-02 | `[100,200,300,399]`, period 3, multiplier 2 | Baseline 200, RVOL 1.995, false; UI rounding cannot make it true |
| P1-OR-03 | `[0,0,0,100]` / `[100,100,100,0]` | First: null/ZERO_BASELINE; second: RVOL 0, false/VALID |
| P1-OR-04 | `[100,null,300,400]` / negative or infinite volume | Null/INVALID_VOLUME; do not shrink the window or replace missing with zero |
| P1-OR-05 | Every prefix; append/mutate only future bars | Already available values, quality and reasons match a full-series prefix; early warmup remains unavailable |
| P1-OR-06 | Period 20 with 20 total bars, then bar 21 | First result unavailable; first calculable result is zero-based index 20 when all volumes valid and baseline positive |
| P1-OR-07 | Valid spike true/false and unavailable spike, evaluated through AST and explicit-object DSL | Valid cases agree. Unavailable produces unavailable even under `not`, OR with a true branch, or equality to false; adapter does not call the evaluator on missing dependencies |
| P1-OR-08 | `volume.spike`, unknown alias, arbitrary call/import/attribute expression, duplicate signal ID, invalid params | Explicit rejection; no widened grammar, silent overwrite or default substitution |
| P1-OR-09 | Temporary DB has five bars; replay index 3; client asks for out-of-scope index/range; then rewind to 2 | Valid response has no index >3. Extra override fields rejected. Rewind yields only <=2 and no retained t=3 signal/explanation |
| P1-OR-10 | Delay response for index 3; change index/symbol/config; return old response last | UI discards old response and displays only matching context or loading state |
| P1-OR-11 | Registry defaults omitted versus explicit; same inputs twice | Equal resolved params, canonical hash and results; param/version changes alter the relevant identity |
| P1-OR-12 | Registry → configure → inspect warmup/true/false/error states at 1440×1000 | UI text/numbers agree with API, no console/page errors, no future signal in response or rendered state, no new Trade/P&L output |

Map these slice-local IDs to Master `SIG-VOL-001/002`, `FR-CORE-003/004/006/007/011`, `TEST-SIG-001/002/003`, `TEST-DSL-001`, `TEST-CAUSAL-001`, and repository G-01..05/R-01/R-04. Slice IDs supplement existing acceptance; they do not redefine it. The malformed-DSL and negated-missing cases are required regression guards.

### P.6 Verification and evidence contract

This review edits documentation only; it does not claim new test/UAT passes. M0 records the current gate baseline. In the implementation batch, use a fresh PowerShell process with a verified temporary `DATABASE_URL` for focused backend tests; never import `app.main` against the default production database merely to inspect a schema. Validate destructive cleanup targets stay inside the created temporary directory. Existing Windows wrappers already configure temporary databases but must still be checked for drift, port conflicts and failed seeding before execution.

Commands below are the future batch's verification contract. Focused filenames are planned and become runnable only when implemented; verify wrapper/script paths before use.

```powershell
# Repository root, fresh PowerShell process; keep this temporary path as evidence.
$signalTestDir = Join-Path ([IO.Path]::GetTempPath()) ('sumi-p1-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $signalTestDir -ErrorAction Stop | Out-Null
$signalTestDb = (Join-Path $signalTestDir 'tests.db').Replace('\', '/')
$env:DATABASE_URL = 'sqlite:///' + $signalTestDb
Push-Location backend
try {
  & .\.venv\Scripts\python.exe -m pytest app/tests/test_signals.py app/tests/test_signal_binding.py app/tests/test_signals_api.py -q
  if ($LASTEXITCODE -ne 0) { throw 'Phase 1 backend tests failed' }
} finally {
  Pop-Location
  Remove-Item Env:DATABASE_URL -ErrorAction SilentlyContinue
}
Push-Location frontend
try {
  npm.cmd run test -- src/components/signals/__tests__/SignalInspector.test.tsx
  if ($LASTEXITCODE -ne 0) { throw 'Phase 1 component tests failed' }
} finally { Pop-Location }
# Repository root, in separate fresh shells; UAT wrappers run sequentially (same ports).
.\scripts\verify-v2.ps1
.\scripts\run-comprehensive-uat.ps1
.\scripts\run-product-uat.ps1
```

The Windows product wrapper is the platform counterpart of `run-product-uat.sh` named in PLANS.md; record that substitution in the ExecPlan. Comprehensive UAT alone does not replace product acceptance UAT. Confirm each process exit code, the result's actual scenario assertions and artifact freshness; a screenshot or a printed PASS without the required assertion is insufficient. Preserve each run's results/screenshots before a rerun overwrites its directory. Review 1440×1000 screenshots and applicable compact-layout regressions.

Completion evidence includes baseline manifest identity, incremental changed-file list, acceptance-to-test map, exact commands/exit codes/counts, full machine-readable results, screenshots actually reviewed, database hash comparison, known pre-existing failures, and reviewer verdict. The reviewer inspects tests for the bugs in P.1, not only their pass counts. A failed in-scope check or unexplained baseline drift prevents completion. Out-of-scope pre-existing failures stay explicit and cannot be turned into a green full-gate claim.

### P.7 Bounded delegation protocol for all later phases

Never dispatch “implement Phase 0–10” or “finish the plan.” Each assignment names one vertical capability, approved baseline, exact allowed/protected files, input/output/error contracts, requirement IDs, at least one hand-calculated expected result, exact verification commands, rollback limited to the batch's own edits, and stop conditions. The DEV agent implements; a separate reviewer inspects the finished diff and evidence. Reviewer and DEV do not concurrently edit shared files. This document requests no new agent/task by itself.

Before later task cards become executable packets, split P3 into individual pattern/trigger/regime or support-resistance deliveries; P8 into individual Ichimoku, confirmed-pivot/divergence and composite families; P2 into kernel, market-rule profile and service integration milestones. A phase number is an ordering label, not the maximum work an agent must attempt in one context.

The reviewer must freeze these task-local semantics before their respective batch: P2 exit precedence/gap fills/settlement and event timing; P3 pattern equality/confirmation/warmup; P4 all nine metric signs, denominators and open-position policy; P5 method/value-source boundaries and invalid denominator cases; P7 membership/coverage/aggregation gates; P8 pivot tie-breaking/confirmation, projection alignment and any promoted threshold versions. Use the relevant authoritative spec excerpts; a field still marked candidate/TBD cannot be implemented as a production default by agent inference.

Stop and return evidence when source requirements conflict, a protected file must change, expected arithmetic disagrees with implementation, an unplanned dependency/migration is needed, or concurrent edits invalidate the baseline. Do not remove assertions, rewrite requirements to match output, perform broad cleanup or implement the next phase to make the current phase pass.

Copyable first-DEV prompt, **only after separate implementation authorization and P0 readiness**:

> Execute only `P1-SIG-01 + P1-SIG-02` in the current authorized checkout. Read AGENTS.md, PLANS.md, the accepted baseline record, and sections O/P of `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`, plus the referenced Master signal contracts and repository acceptance IDs. Create one ExecPlan that copies the frozen P.4 contracts, P.5 expected cases and P.6 verification commands with exact temporary-environment setup. Complete M0–M4 as one vertical capability. Implement only Relative Volume/Volume Spike, registry, reasons, replay-scoped API/UI and isolated safe DSL binding. Preserve the captured existing work. Do not connect new signals to legacy automated execution or evaluate their P&L. Return incremental diff, mapped test/UAT evidence, reviewed screenshots, database verification and limitations. Stop after reviewer handoff; Phase 2 is not authorized by this prompt.

## Planning Definition of Done

- Original Price/Volume/Money Flow and strategy/backtest scope is represented.
- Latest BB v3 and verified Doraemon capability are reconciled without promoting candidates.
- SUMI-420 is narrowed from full-market scope without falsifying historical membership.
- Every implementation task records dependencies, expected code surfaces, persistence/API/UI impact, tests, acceptance, DoD, risk and migration.
- Data acquisition is a parallel non-blocking track.
- Current code freshness changes are recorded and user-owned work is preserved.
- No production code, migration, database or behavior was changed by this planning task.

**STOP:** Review and authorize a bounded DEV slice separately. Do not begin implementation from this planning task.
