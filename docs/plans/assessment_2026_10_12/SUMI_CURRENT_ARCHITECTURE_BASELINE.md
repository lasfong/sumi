# SUMI Current Architecture Baseline — 2026-10-12

**Scope:** read-only recon. No code/test/DB/migration changes, no servers, no `backend/sumi.db` access.
**Authority order:** `AGENTS.md` > `PLANS.md` > ratified ADRs (`docs/architecture/decisions/`) > `TARGET_ARCHITECTURE_V3.md` > `CURRENT_STATE_ARCHITECTURE_REVIEW.md` / `SUMI_GAP_REGISTER.md` > handoff/reports (provenance only).
**Labels:** VERIFIED = seen in file this pass. REPORTED = doc claims, not re-verified. INFERRED = reasoned from evidence. UNKNOWN = not checked / needs runtime. PROPOSED = this assessment's suggestion (only in files 5–6).

**Working tree at pass start:** 17 modified files + 5 untracked test files + untracked `docs/architecture/`, `docs/plans/`, `docs/testing/` (see §8). All findings below are pre-commit unless noted.

## 1. System context (VERIFIED structure, REPORTED behavior)

```
FE (Vite + React 19.2.6 + TS + TanStack Query 5.101 + Zustand 5 + LWC 5.2 + react-router 7)
  → REST /api/* + WS /api/ws/replay/:id
  → FastAPI services → domain → SQLite backend/sumi.db
  → EXT: CafeF CSV/ZIP (real EOD) + SSI/vnstock STUB (synthetic) + Binance/Yahoo (dead)
```

- Backend routes VERIFIED (`backend/app/api/`): `analytics, backtest, bb, decisions, health, import_data, indicators, journal, replay, scanner, signals, strategy_lab, symbols, sync, universe, ws_replay` (16 modules).
- Domain VERIFIED (`backend/app/domain/`): `engine/` (broker, cache, events, indicator_engine, market_constraints, models, strategy, strategy_indicator_adapter), `backtest/` (batch_runner, execution, metrics, models, persistence_adapter), `market/` (calendar, provider, rules, settlement) + `market_rules.py`, `signals/` (15 modules: candle_features, divergence, flow_events, health, ichimoku, models, patterns, regimes, registry, structure, support_resistance, technical, volume, vsa), `strategy/` (5 modules), `bb/` (calculator, chart_adapter, contracts, market_aggregator), `data/` (contracts, ports), `data_feeds/` (base, binance_feed, yahoo_feed), `regime/`, `universe/`, `trade_planning.py`, `accounting.py`, `enums.py`.
- Models VERIFIED (`backend/app/models/`): candle, decision, drawing, event_log, execution, import_run, journal_entry, order, position, replay_session, scanner_run, strategy_lab_run, symbol, sync_run, trade (+ `__init__`). REPORTED 20 tables / head `a2d639ecd5d5` (CURRENT_STATE review); live counts REPORTED (2366 candles / 1183 symbols / 4 import_runs / 0 sync_runs / 0 replay_sessions) — not re-verified (DB off-limits).
- Services VERIFIED (partial list): `analytics_service, backtest_cleanup_service, backtest_service`, + `replay_service, practice_workflow*, trade_lifecycle_service, cafef/*, classifier, import_workflow_service, sync_workflow_service, weekly_aggregator, data_providers/` (base, provider_registry, ssi_provider, vnstock_provider), `strategy_lab_service, scanner*, signal*, bb_service, batch*`. Full service inventory UNKNOWN (listing capped at 120 files).
- Frontend VERIFIED: pages `Analytics, Backtest, Dashboard, Import, Journal, Replay, Scanner, StrategyLab` (+ css/test); components dirs `analytics, chart, common, layout, replay, settings, signals, strategy, sync`; replay dir contains `ReplayWorkspace.tsx, ReplayWorkspaceController.tsx` (god pair REPORTED 764+602=1366 lines — line counts not re-verified), `TradeControls, PositionPanel, PendingOrdersPanel, PracticeRail/Journal/Scoreboard, ReplayControlDock, SessionSetup, SymbolSwitcherModal, DecisionJournal, ScannerSourceContext, SessionDebriefModal, KeyboardShortcutsModal`; deps VERIFIED `frontend/package.json:16-24` (react 19.2.6, query 5.101, lightweight-charts 5.2, zustand 5.0.14, router 7.17).
- Tests VERIFIED (filenames): `backend/app/tests/` ~50 files incl. `test_replay_no_future_leak, test_backtest_kernel, test_batch_runner, test_indicator_parity_e2e, test_market_data_contracts, test_practice_workflow, test_trade_lifecycle, test_ws_replay, test_scanner_replay_integrity, test_sync_workflow` + untracked `test_b01_transaction_fk, test_b02_quarantine_isolation, test_b02_quarantine_write_collision, test_b02_synthetic_deny`; `backend/tests/` has `test_cwd_database_isolation, test_t2_settlement, e2e_test, screenshot_test`; `docs/testing/` has `ACCEPTANCE_TRACEABILITY_MATRIX, CONTRACT_TEST_STRATEGY, FINANCIAL_GOLDEN_CASES, MASTER_TEST_STRATEGY, PAGE_TEST_SCENARIOS, WEBSOCKET_TEST_MATRIX` (all VERIFIED present, contents mostly REPORTED).
- Canonical docs VERIFIED present: `AGENTS.md`, `PLANS.md` (65 lines), `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`, `docs/architecture/TARGET_ARCHITECTURE_V3.md`, `SUMI_GAP_REGISTER.md` (P0 17 + P1 ~40 + P2 list), `CURRENT_STATE_ARCHITECTURE_REVIEW.md`, `DEEP_ARCHITECTURAL_AUDIT_REPORT.md`, 8 ADRs in `docs/architecture/decisions/`, `docs/plans/SUMI_V3_MASTER_REFACTOR_PLAN.md` (78 lines), `docs/plans/DEV_BATCHES.md` (58 lines), 15 prompts in `docs/plans/prompts/`, `docs/dev-program/STATE.json` (status `complete`, 20 tasks accepted), `docs/testing/FINANCIAL_GOLDEN_CASES.md` (12 cases, head VERIFIED), `docs/release/V3_ACCEPTANCE_MATRIX_AND_LIMITATIONS.md` (P-series matrix, head VERIFIED), `scripts/verify-v2.sh` (temp-DB gate VERIFIED header).

## 2. Data flows

### 2.1 CafeF import → persistence (VERIFIED paths, REPORTED semantics)
`ImportPage` → `api/import_data` → `cafef/* + classifier + import_workflow_service` → preview → accept → `Candle` + `ImportRun/Item/Mutation` + provenance → `weekly_aggregator` derives 1W locally. Design strengths REPORTED: checksum + stale-revalidate + mutations + provenance + weekly-local. Safety gaps VERIFIED by reference: bare `commit/flush` without rollback/tutorial (GAP-DB-02 cites `import_workflow:53,101,214,280,328,379,405,469` — line numbers REPORTED, pattern INFERRED from diffstat showing 377-line rewrite in working tree); preview-commit REPORTED (GAP-DB-14). **Working-tree note:** `import_workflow_service.py` (+377/−) and `weekly_aggregator.py` (+52) are modified uncommitted — scope INFERRED as B00–B02 safety rework, content UNKNOWN (not diffed line-by-line this pass).

### 2.2 Sync provider → quarantine (VERIFIED new guard in working tree)
- Stub synthesis VERIFIED: `ssi_provider.py:32` + `vnstock_provider.py:32` set `is_synthetic=True`; `md5+sin` wave at `ssi:96,100` / `vnstock:90,93`.
- Guard VERIFIED in working tree: `sync_workflow_service.py:63` propagates `is_synthetic`; `:172-192` preview path; `:268-292` stores flag; `:344-346` `if is_synthetic and not settings.SUMI_ALLOW_SYNTHETIC_PROVIDERS` deny; `:378-379` warning `Dữ liệu giả lập (quarantine dataset)`; `:386` quarantine branch; `:431-433` `candle_source = quarantine:sync:{id}` + `dataset_type = quarantine|canonical`. **This matches ADR-PROV-001 + B02 spec — but it is UNCOMMITTED and UNACCEPTED.** Live `sync_runs=0` REPORTED (pre-guard). Do not treat as closed.
- Config VERIFIED modified: `config.py` (+66), `db.py` (+94), `dependencies.py` (+4), `alembic/env.py` (+3) — INFERRED B00/B01 path-absolute + FK-pragma + rollback rework. Untracked `test_cwd_database_isolation.py` VERIFIED header asserts absolute `backend/sumi.db` anchoring + relative-URL normalization; untracked `test_b01*` VERIFIED header asserts FK listener + isolated file DB; untracked `test_b02_synthetic_deny` VERIFIED header builds fake real/synthetic adapters. **Results UNKNOWN — pytest forbidden this pass.**

### 2.3 Replay cursor (VERIFIED core slice, VERIFIED edge leaks)
- Core slice VERIFIED: `replay_service.py:168-169` comment + `limit = current_index + 1`; navigation `:218-226` (`destination min(current+steps, final)`, loop advances index), rewind `:313-314`; session create `:117` `current_index=0`; `test_replay_no_future_leak.py:12-50` asserts 1→2→3→2 candles (VERIFIED header).
- Edge leaks VERIFIED: `api/decisions.py:42,49,66,70` raw `.all()` by session with no `candle_index` predicate (grep VERIFIED). Indicator projection REPORTED: `indicator_engine.py:349` `reindex(new_index)` (VERIFIED line exists; future-open semantics REPORTED via GAP-REPLAY-02, `api/replay:152-171` append-all REPORTED). Forward meta REPORTED (`total_bars/earliest_release_date` in practice payloads). So: **core REST candle path enforces cursor; ledger/indicator/meta edges do not (pre-B10/B11).**

### 2.4 Scanner / Strategy Lab / Backtest (REPORTED, paths VERIFIED to exist)
- `api/scanner.py` + `api/strategy_lab.py` + `api/backtest.py` + `domain/backtest/{execution,metrics,batch_runner}` + `domain/strategy/*` all VERIFIED present. Semantics REPORTED: dual timing (legacy same-close `backtest_service:312,325-331` + `trade_lifecycle:251,337` vs kernel next-open `execution:127-130` + intraday-same `execution:422-660`); sweep-always-legacy (`strategy_lab:282-291`); slippage/T+2/PnL/winner/PF/Sharpe multi-truth (GAP-EXEC-02..07). **INFERRED: one kernel unification (B20–B22) is the highest-leverage correctness work after safety + lookahead.**
- Signal domain VERIFIED present (15 modules) + P-series reviews REPORTED accepted (STATE.json `complete`). FE signal UIs VERIFIED dirs (`components/signals`, `components/strategy`); version/payload drift REPORTED (GAP-API-07, GAP-FE-08).

### 2.5 WS playback (REPORTED, file VERIFIED)
`api/ws_replay.py` + `useWebSocket` VERIFIED present. REPORTED: no reconnect/backoff/heartbeat/validate, speed default skew, fire-and-forget sends, missing error/completed frames, prefix-cache wrong-TF + optimistic `current_index+1` in Controller. Chaos matrix doc VERIFIED present (`WEBSOCKET_TEST_MATRIX.md`), results UNKNOWN.

## 3. Ownership of business rules (target per TARGET §1–2 + ADRs; current REPORTED)

| Rule | Target owner | Current (REPORTED unless noted) |
|---|---|---|
| Cursor authority | BE `replay_sessions.current_index` | Core BE (VERIFIED slice); FE optimistic +1, WS echo missing |
| Indicator math | BE `IndicatorEngine` only | BE-only upheld; projection cut missing (B11) |
| Execution timing/fill | one next-open kernel + `MarketProfile` | dual legacy/kernel + intraday exception (B20) |
| Fee/slippage/tax/lot | `MarketProfile` single source | 3 slippage truths (B21) |
| Settlement T+1.5 calendar | `SettlementEngine` | calendar vs bar-count dual (B21) |
| Realized PnL | net everywhere | gross/net mixed (B22) |
| Metrics (winner/PF/gate/Sharpe/DD) | one spec | 3-way drift (B22) |
| Synthetic policy | default-deny + quarantine | **guard in working tree (VERIFIED), unaccepted** |
| Tx/FK/DB path | absolute + rollback + pragma | **rework in working tree (VERIFIED modified), unaccepted** |
| Contracts | envelope + snapshot + one catalog/signalsClient | 11+4 mismatches (B40/B41) |
| Chart calls | `ChartKernel` facade | scattered `createChart` (B32) |
| Session authority | URL `?session=` | URL + Query + half-dead Zustand + useState (B30) |
| Drawings | versioned provider-independent 7-kind | 7-kind custom + 3 dead legacy files (B32) |

## 4. API / FE-BE boundaries
- 16 API modules VERIFIED; `response_model` coverage UNKNOWN (GAP claims missing on BB GET — REPORTED).
- Error envelope: `detail=str(e)` REPORTED scattered (`replay:142, indicators:90, analytics:14, bb:68,106, universe:28,40,42`); FE 422-array→`[object Object]` REPORTED (`client:12-20`, `DataSyncPanel:16-19`).
- Dead/ambiguous routes REPORTED: `/api/import/catalog` vs `/symbols/catalog`; `importCafefData` always-400; `data_feeds/*` + `DataFeedService` bypass; `/backtest` orphan route (`App:57`) with no Sidebar entry.
- AGENTS.md boundaries vs current (REPORTED gaps, consistent with TARGET): business logic in routes (needs per-route audit — UNKNOWN this pass); `ReplayPage` as god composition (REPORTED 1366 lines); chart calls outside facade (REPORTED); indicator/drawing state serializability (drawing schema doc VERIFIED present `decision-packs/sumi-drawing-document-v1.schema.json`, runtime conformance UNKNOWN).

## 5. Duplicated / dead paths (VERIFIED existence, REPORTED deadness)
`domain/data_feeds/{binance_feed,yahoo_feed}` + `DataFeedService` (files VERIFIED, bypass REPORTED); 3 legacy drawing files (names REPORTED, import-graph not re-verified); dual catalog routes (REPORTED); dual execution paths (REPORTED); Zustand half-dead store (dep VERIFIED, dead-keys REPORTED); `SumiPrimitive` type-only residue (REPORTED as good); P2 perf list (vectorize/BETWEEN/bulk/paginate/duplicates — REPORTED).

## 6. Feature completeness (summary; detail in file 2)
No feature is IMPLEMENTED end-to-end to acceptance (browser UAT evidence UNKNOWN/absent this pass). Strongest REPORTED areas: signal registry research surface (P-series accepted), import preview/accept design, replay core slice, indicator backend authority, 7-tool drawing. All carry P0/P1 gaps.

## 7. Deltas vs documented target
Working tree **advances toward** TARGET §4 (Tx/safety) and §4-synthetic (quarantine) — VERIFIED guard fragments. Everything else in TARGET §§2–3, 5–7 (kernel, replay lockdown, WS, contracts, god-split, UX) shows **no VERIFIED implementation delta** this pass (modified files are all data/sync/config/verify-script + sync FE panel). GAP_REGISTER status: 0 P0 FIXED can be claimed — acceptance requires tests + UAT + reviewer, none available read-only.

## 8. Worktree evidence (VERIFIED `git status --short` + `git diff --stat`)
Modified (17): `backend/alembic/env.py, app/config.py, app/db.py, app/dependencies.py, app/main.py, app/schemas/sync_schema.py, app/services/data_providers/{base_provider,provider_registry,ssi_provider,vnstock_provider}, app/services/{import_workflow_service,sync_workflow_service,weekly_aggregator}, app/tests/test_sync_workflow.py, frontend/src/{api/syncApi.ts,components/sync/DataSyncPanel.tsx}, scripts/verify-v2.sh` (899+/307−). Untracked: `test_b01_transaction_fk, test_b02_quarantine_isolation, test_b02_quarantine_write_collision, test_b02_synthetic_deny, backend/tests/test_cwd_database_isolation` + `docs/architecture/, docs/plans/, docs/testing/`. HEAD `1ff5f04`. `docs/plans/assessment_2026_10_12/` created this pass (new deliverables only).
