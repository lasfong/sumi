# SUMI V3 — DEVELOPMENT EXECUTION STATE

- **Active Train**: **Development Train 04 (`Wave W4`: `B40 → B41`, `B50 → B51`) — COMPLETE & VERIFIED IN WORKING TREE**
- **Prior Train Status**: **Development Train 01 (`W0-A`, `B03`, `B10`, `B11`), Train 02 (`B20`, `B21`, `B22`), Remediation (`R1–R7`), & Train 03 (`B30`, `B31`, `B32`) COMPLETE & COMMITTED (`e34516c`, `2151f34`, `c331dc3`, `0732ca0`, `12e166a`)**
- **Current Phase**: **DEVELOPMENT TRAIN 04 COMPLETE — READY FOR GPT-6 REVIEW**
- **Baseline Git Commits**:
  - Base: `1ff5f04ed7a26969130eaba75da50b20684e763e` (`master`)
  - Commit 1 (`W0-A` Stabilization, 38 files): `e34516c`
  - Commit 2 (Architecture Baseline & Master Plan, 46 files): `2151f34`
  - Commit 3 (`W0–W2` Execution & Financial Correctness Baseline, 70 files): `c331dc3` (`c331dc3250937a664625b1ebb9e7fac6170b0ae3`)
  - Commit 4 (Persistent Project Execution Charter, 4 files): `0732ca0` (`0732ca07cd2cb3fcf6f45bef85d93f8fd8bdcb21`)
  - Commit 5 (`W3` Replay State, WebSocket & Chart Integration Baseline, 31 files): `12e166a` (`12e166ad323437b43607c6ba739f4b273a7a4d21`)
- **Production DB Invariant (`backend/sumi.db`)**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes — verified bit-exact, `0` byte delta)

---

## Development Train 01 Summary (`COMPLETE & COMMITTED`)

1. **Phase A — `W0-A` (`B00`, `B01`, `B02` + `DB-SAFETY`)**: `COMPLETE_COMMITTED` (`e34516c`, `2151f34`; `151/151` tests passed in `artifacts/local/development-train-01/w0a_140_regression.log`).
2. **Phase B — `B03` (`Schema, Migration Compatibility & Preview Isolation`)**: `COMPLETE_COMMITTED` (`c331dc3`; `14/14` tests passed in `artifacts/local/development-train-01/b03_tests.log`).
3. **Phase C — `B10` (`Replay Ledger, Horizon & Eligibility Zero-Lookahead Lock`)**: `COMPLETE_COMMITTED` (`c331dc3`; `8/8` tests passed in `artifacts/local/development-train-01/b10_tests.log`).
4. **Phase D — `B11` (`Indicator Forward Projection Clamping & Typed Metadata Contract`)**: `COMPLETE_COMMITTED` (`c331dc3`; `9/9` tests passed in `artifacts/local/development-train-01/b11_tests.log`).

---

## Development Train 02 Batch Status (`Wave W2`: `B20 → B21 → B22` — `COMPLETE & COMMITTED`)

### 1. Phase A (Train 02) — `B20` (`Execution Kernel Parity: Next-Open Across All Paths & Fill-Estimate Sizing`)
- **Scope**:
  - Close `GAP-EXEC-01` and `GAP-EXEC-07` per `ADR-EXEC-001`, `DEC-03`, and `DEC-04`.
  - Unify Parameter Sweep (`backend/app/services/strategy_lab_service.py`) and Single Backtest (`backend/app/services/backtest_service.py`) onto `BacktestExecutionKernel` (`backend/app/domain/backtest/execution.py`) so `Signal[T] → Order QUEUED MARKET_NEXT_OPEN → Fill Open[T+1]` and `T = N - 1 → UNFILLED_AT_CUTOFF` (`G-01`, `G-03`, `G-04`, `G-05`, `G-11`, `G-12`).
- **Implementation Status**: `COMPLETE_COMMITTED` (`c331dc3`)
- **Test Evidence**: `11/11 passed` in `backend/tests/test_b20_execution_kernel_parity.py`.

### 2. Phase B (Train 02) — `B21` (`MarketProfile Single Source, T+1.5 Settlement & VN Calendar`)
- **Scope**:
  - Close `GAP-EXEC-02`, `GAP-EXEC-03`, `GAP-DB-08`, `GAP-DB-11`, and `GAP-U-01` (wording contract) per `ADR-SETTLE-001`.
- **Implementation Status**: `COMPLETE_COMMITTED` (`c331dc3`)
- **Test Evidence**: `14/14 passed` in `backend/tests/test_b21_settlement_and_calendar.py`.

### 3. Phase C (Train 02) — `B22` (`Financial Metrics Unification: Net PnL, Win Rate, PF N/A, Sample Gate & Sharpe 252`)
- **Scope**:
  - Close `GAP-EXEC-04`, `GAP-EXEC-05`, and `GAP-EXEC-06` per `ADR-EXEC-001`, `TARGET_ARCHITECTURE_V3.md` §2, and `DEC-05`.
- **Implementation Status**: `COMPLETE_COMMITTED` (`c331dc3`)
- **Test Evidence**: `10/10 passed` in `backend/tests/test_b22_metrics_unification.py`, `11/11 passed` in `test_benchmark_metrics.py`.

### 4. Train 01+02 Remediation & Baseline Closure (`R1–R7` — `COMPLETE & COMMITTED`)
- **Commit Hash**: `c331dc3250937a664625b1ebb9e7fac6170b0ae3` (`feat(v3): complete W0-W2 execution and financial correctness baseline`)
- **Governance Charter Commit Hash**: `0732ca07cd2cb3fcf6f45bef85d93f8fd8bdcb21` (`docs(governance): establish Sumi V3 execution charter`)
- **Verification Summary**: `828 passed, 1 skipped, 0 failed` backend; `240 passed` frontend.

---

## Development Train 03 Batch Status (`Wave W3`: `B30 → B31 + B32` — `COMPLETE & COMMITTED`)

### 1. Phase A (Train 03) — `B30` (`Frontend Replay State, TanStack Query Keys, Navigation Race Protection & Session Authority`)
- **Scope & Traceability**:
  - Close `GAP-FE-01`, `GAP-FE-02`, `GAP-FE-06`, and `GAP-FE-09` per `ADR-FE-001` (`docs/architecture/decisions/ADR-FE-001-state-ownership.md`) and `docs/plans/prompts/B30_fe-state.md`.
- **Implementation Status**: `COMPLETE_COMMITTED` (`12e166a`)
- **Test Evidence**: `6/6 passed` in `frontend/src/features/replay/__tests__/b30_fe_state_race.test.tsx`.

### 2. Phase B (Train 03) — `B31` (`WebSocket Sequencing, Heartbeat, Reconnect & State Resynchronization`)
- **Scope & Traceability**:
  - Close `GAP-WS-01` and `GAP-FE-10` per `ADR-WS-001` (`docs/architecture/decisions/ADR-WS-001-ws-reliability.md`), `docs/testing/WEBSOCKET_TEST_MATRIX.md` (`WS-01..WS-10`), and `docs/plans/prompts/B31_ws.md`.
- **Implementation Status**: `COMPLETE_COMMITTED` (`12e166a`)
- **Test Evidence**:
  - Backend: `6/6 passed` in `backend/tests/test_b31_ws_replay.py` (including real disconnect & reconnect sequence counter reset test) + `10/10 passed` in `backend/app/tests/test_ws_replay.py` & `test_scanner_replay_integrity.py`.
  - Frontend: `7/7 passed` in `frontend/src/hooks/__tests__/b31_ws_chaos_matrix.test.tsx` (including real socket close & new connection `seq=1` acceptance + queued command ordering).
  - Live Browser Gate A1 (`b31.ws-real-connection-termination-and-reconnect-gate`): Verified live WebSocket termination (`connId=4`, `preDropSeqs=[1..7]`), new connection establishment (`connId=5`, `postDropSeqs=[1..9]` accepted without discard), ordered queued command flush (`sync`, `set_speed=100`, `start`, `pause`), and full cursor/candle resync (`#49 / 349`, `49/49` unique ordered candles).

### 3. Phase C (Train 03) — `B32` (`ChartKernel Facade, ReplayWorkspace Restructuring, Indicator Rendering Contract, Drawing & Chart Stability`)
- **Scope & Traceability**:
  - Close `GAP-FE-03`, `GAP-FE-04`, `GAP-FE-05`, `GAP-FE-07`, and `GAP-FE-08` per `ADR-CHART-001` (`docs/architecture/decisions/ADR-CHART-001-chart-kernel.md`) and `docs/plans/prompts/B32_chart.md`.
- **Implementation Status**: `COMPLETE_COMMITTED` (`12e166a`)
- **Test Evidence**: `3/3 passed` in `ChartKernel.test.tsx`, `6/6 passed` in `IndicatorContract.test.ts`, `9/9 passed` in `ReplayPage.test.tsx`.

### 4. Phase D (Train 03) — Gate A1 & Gate A2 Baseline Commit (`COMPLETE & COMMITTED`)
- **Commit Hash**: `12e166ad323437b43607c6ba739f4b273a7a4d21` (`feat(v3): complete W3 replay state websocket and chart integration`) — 31 verified `W3` paths (`19` modified, `4` deleted legacy drawing files, `8` new source/test files).
- **Verification Summary at Commit**: `834 passed, 1 skipped, 0 failed` backend; `253 passed, 0 failed` frontend; `17/17` live browser smoke checks (`artifacts/local/development-train-03/smoke/train03-smoke-results.json`).

---

## Development Train 04 Batch Status (`Wave W4`: `B40 → B41`, `B50 → B51` — `COMPLETE & VERIFIED IN WORKING TREE`)

### 1. Batch `B40` — `API Contract & TypeScript Alignment`
- **Scope & Traceability**:
  - Close `GAP-API-01`, `GAP-API-03`, `GAP-API-04`, `GAP-API-05`, `GAP-API-06`, `GAP-API-07`, `GAP-API-08`, `GAP-API-09`, `GAP-API-10`, `GAP-API-11` (Candle & Scanner part), `GAP-DB-07`, `GAP-DB-13`, and `GAP-FE-08` per `ADR-API-001` (`docs/architecture/decisions/ADR-API-001-contract-governance.md`) and `docs/plans/prompts/B40_contracts.md`.
  - Standardized global error envelopes (`{code, message, field_errors, detail}`) in `backend/app/main.py` (`RequestValidationError`, `HTTPException`, `Exception`) and stripped raw `str(e)` internal exception leaks in `analytics.py`, `universe.py`, and `indicators.py`.
  - Normalized HTTP 422 array/object errors in `frontend/src/api/client.ts` (`extractErrorMessage`) so UI toasts never render `[object Object]`.
  - Aligned enums & DTOs across backend/frontend: `SessionStatus` includes `'bankrupt'`; `DecisionCreate` validator permits `OrderType.CUSTOM_PRICE`; `CreateSessionMode` (`normal | random | blind_symbol | blind_date`) is strictly separated from `SessionMode` (which adds `backtest` for read/response DTOs) with a client-side guard in `replayApi.createSession` and HTTP 422 rejection on `POST /api/replay/sessions`; `TradePlanResult` includes all 7 risk fields (`entry_price`, `stop_loss`, `target_price`, `direction`, `account_equity`, `available_cash`, `risk_percent`); `AnalyticsReport` optionality aligned; `checklist_snapshot` supports structured dict or legacy JSON string across `decision_schema.py`, `trade_lifecycle_service.py`, `decision.ts`, and `journal.ts`; `Candle.source: string | null` added; explicit `response_model` annotations added on `/api/scanner/*` (`ScannerRunResponse`, `ScannerHistoryItemResponse`).
  - Enforced client-side signal contract invariants (`1..2` signals max, unique signal names, non-null `params`, pinned `version`, no extra fields) in `frontend/src/api/signalsApi.ts`, `SignalExplanationInspector.tsx`, and `SignalInspector.tsx`.
  - Removed duplicate unannotated `GET /api/import/catalog` in `backend/app/api/import_data.py` (retaining canonical `GET /api/data/catalog` in `symbols.py`), removed dead `importCafefData` in `frontend/src/api/importApi.ts`, and isolated legacy `DataFeedService` in `backend/app/services/data_service.py`.
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - Backend: `11/11 passed` in `backend/tests/test_b40_openapi_contract_snapshot.py`.
  - Frontend: `17/17 passed` in `frontend/src/api/__tests__/b40_contract_alignment.test.ts`, `SignalExplanationInspector.test.tsx`, and `SignalInspector.test.tsx`.
  - Browser Smoke: `b40.openapi-and-error-envelope-contract` passed.

---

### 2. Batch `B41` — `Bollinger / Money Flow Indicator Completion`
- **Scope & Traceability**:
  - Close `GAP-API-02` and `GAP-API-11` (BB part) per `ADR-API-001` and `docs/plans/prompts/B41_bb.md`.
  - Added explicit Pydantic `response_model` declarations (`BBHorizonsResponseSchema`, `Union[BBSymbolResponseSchema, BBChartFormatResponseSchema]`, `MarketBBResponseSchema`) to all `/api/bb/*` GET routes in `backend/app/api/bb.py` and `backend/app/schemas/bb_schema.py`.
  - Unified `series` (`format=series` default) and `chart` (`format=chart` flattened `ChartAdapter.to_chart_series`) contracts in `frontend/src/api/bbApi.ts` (`fetchSymbolBB`, `fetchSymbolBBChart`) and `frontend/src/components/chart/TechnicalFlowBBViewer.tsx`.
  - Distinguished warming (`is_warmup`), degraded (`UNAVAILABLE_DEGRADED` / low data quality / breadth `< 50%`), invalid (`ERROR`), and genuinely empty states with explicit Vietnamese banners and reasons in `TechnicalFlowBBViewer.tsx`, while rendering valid BB series through `ChartKernel` (`CandleChart`).
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - Backend: `32/32 passed` in `backend/tests/test_b41_bb_contract.py`, `backend/app/tests/test_bb_calculator.py`, and `backend/app/tests/test_market_bb.py`.
  - Frontend: `5/5 passed` in `frontend/src/components/chart/__tests__/TechnicalFlowBBViewer.test.tsx`.
  - Browser Smoke: `b41.bb-series-and-chart-mode-contract-and-ui` passed.

---

### 3. Batch `B50` — `Trading Practice UX`
- **Scope & Traceability**:
  - Close `GAP-U-01`, `GAP-U-02`, `GAP-U-04` (pending `LIMIT` cancel part), `GAP-U-11` (Trade dialog `MarketProfile` assumptions), and `GAP-U-12` (`CLOSE` vs `SELL` tooltip clarity) per `ADR-SETTLE-001`, `ADR-EXEC-001`, and `docs/plans/prompts/B50_trading-ux.md`.
  - Added `available_quantity`, `unsettled_quantity`, `earliest_release_date`, and `settlement_label` to `PositionPanel.tsx` (`Khả dụng` vs `Chờ về`) and rendered an always-visible inline `T+1 · về sau 1 phiên` settlement notice (`data-testid="settlement-inline-notice"`) directly below `SELL`/`CLOSE` in `TradeControls.tsx`.
  - Added inline Vietnamese SL/TP geometry validation (`data-testid="sl-tp-validation-alert"`) in `TradeControls.tsx` and `backend/app/domain/trade_planning.py` (`BUY`: `SL < Entry < TP`; `SELL`/`SHORT`: `TP < Entry < SL`) that keeps the sizing panel visible while disabling submission on invalid geometry.
  - Made `SHORT` PnL, R-multiple, and bidirectional `Risk-Reward` chart drawing sync symmetric across `practiceDomain.ts`, `PositionPanel.tsx`, `TradeControls.tsx`, `ReplayWorkspace.tsx`, and `ReplayWorkspaceController.tsx`.
  - Implemented `DELETE /api/replay/sessions/{session_id}/orders/{order_id}` (`cancel_pending_order` in `backend/app/services/trade_lifecycle_service.py` and `backend/app/api/decisions.py`) and wired a visible `Hủy lệnh` (`Cancel`) button in `PendingOrdersPanel.tsx`.
  - Displayed Vietnam `MarketProfile` assumptions (`Phí mua 0.15% · Phí bán 0.15% · Thuế bán 0.10% · Lô chẵn 100 CP · T+1 · về sau 1 phiên`) inside the `TradeControls.tsx` order dialog (`data-testid="trade-dialog-market-assumptions"`).
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - Backend: `4/4 passed` in `backend/tests/test_b50_trading_practice_ux.py`.
  - Frontend: `4/4 passed` in `frontend/src/components/replay/__tests__/b50_trading_practice_ux.test.tsx`.
  - Browser Smoke: `b50.replay-above-the-fold-layout-1440x1000-and-1280x800`, `b50.trade-dialog-assumptions-and-inline-sl-tp-validation`, `b50.pending-limit-order-submission-and-cancellation`, and `b50.market-buy-and-t1-settlement-inline-feedback` passed.

---

### 4. Batch `B51` — `Strategy Lab and Application UX`
- **Scope & Traceability**:
  - Close `GAP-U-03`, `GAP-U-04` (Undo/Redo discoverability), `GAP-U-05`, `GAP-U-06`, `GAP-U-07`, `GAP-U-08`, `GAP-U-09`, `GAP-U-10`, `GAP-U-11`, `GAP-U-12`, and `GAP-U-13` per `docs/plans/prompts/B51_lab-ux.md`.
  - Integrated the full Single Backtest workspace (`BacktestPage`) as the `Kiểm Định Đơn Lẻ (Single Backtest)` tab (`?tab=single`) inside `StrategyLabPage.tsx` with a clear `Single Backtest vs. Strategy Battle` guidance banner (`data-testid="single-vs-battle-banner"`), and redirected `/backtest` in `App.tsx` to `/strategy-lab?tab=single` while preserving Strategy Battle, Parameter Sweep, and Multi-Phase Batch workflows.
  - Completed lossless `StrategyRuleBuilder` → `StrategyLabPage` (`Single` and `Battle`) handoff via `sumi_pending_strategy_rule` (`sessionStorage`), adding a custom AST rule option (`custom_ast_rule.yaml`) to `BacktestPage.tsx`, a prefill banner (`data-testid="custom-rule-prefill-banner"`) with clear button, and toast confirmation.
  - Upgraded Undo/Redo button contrast and tooltips (`Ctrl+Z / Cmd+Z`, `Ctrl+Y / Cmd+Shift+Z`) in `DrawingToolbar.tsx`, documented shortcuts in `KeyboardShortcutsModal.tsx`, and added keyboard handlers + Spacebar interactive focus guard in `ReplayWorkspaceController.tsx`.
  - Updated `Sidebar.tsx` active state matching for `/strategy-lab` query tabs (`single`, `battle`, `multiphase`, `flow` vs `catalog` vs `builder`).
  - Added visible `↑`/`↓` indicator order controls and warming progress badges (`warming (N/X)`) in `IndicatorPaneChrome.tsx` and `ChartLegendOverlay.tsx`.
  - Extracted `Signals` into a dedicated 5th tab in `PracticeRail.tsx` (`Trade`, `Journal`, `Decisions`, `Drawing`, `Signals`) so `BUY`/`SELL`/`CLOSE` stay above the fold at both `1440×1000` (`y=337`) and `1280×800` (`y=337`), added contextual `📘 Nhật ký` / `📊 Phân tích` session links in `ReplayRightPanel.tsx`, and aligned `< 1280px` desktop limitation warnings (`xl:hidden`) and `MarketProfile` assumptions across pages.
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - Frontend: `5/5 passed` in `frontend/src/pages/__tests__/b51_strategy_lab_ux.test.tsx` + `23/23 passed` across `StrategyLabPage.test.tsx`, `BacktestPage.test.tsx`, `StrategyRuleBuilder.test.tsx`, `PracticeWorkflow.test.tsx`, and `ChartKernel.test.tsx`.
  - Browser Smoke: `b51.backtest-redirect-and-single-tab-ux`, `b51.strategy-builder-to-single-and-battle-handoff`, and `b51.indicator-order-controls-and-undo-redo-ux` passed.

---

### 5. Train 04 Full Regression & Live Browser Smoke Summary (`COMPLETE`)
- **Full Backend Pytest Suite (`PYTHONPATH=backend ./.venv/bin/pytest backend/tests backend/app/tests -q`)**: **`843 passed, 1 skipped, 0 failed`** (`844 collected`).
- **Full Frontend Vitest Suite (`npm --prefix frontend run test -- --run`)**: **`268 passed, 0 failed`** across **`43/43` test files**.
- **Frontend Production Build (`npm --prefix frontend run build`)**: **`0 TypeScript/Vite errors`**, built in `275ms`.
- **Live Browser Smoke Suite (`Playwright Chromium`, `1440×1000` & `1280×800`)**: **`12/12 checks PASSED`**, **`0 consoleErrors`**, **`0 pageErrors`** (`artifacts/local/development-train-04/smoke/train04-smoke-results.json`, `strategy-lab-1440x1000.png`, `strategy-lab-1280x800.png`, `replay-w4-1440x1000.png`, `replay-w4-1280x800.png`).
- **Production Database Checksum (`backend/sumi.db`)**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes, `0` byte delta).
- **Status**: **`DEVELOPMENT TRAIN 04 COMPLETE — READY FOR GPT-6 REVIEW`**
