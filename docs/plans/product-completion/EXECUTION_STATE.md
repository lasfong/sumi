# SUMI V3 — DEVELOPMENT EXECUTION STATE

- **Active Train**: **Development Train 03 (`Wave W3`: `B30 → B31 + B32`) — COMPLETE & VERIFIED IN WORKING TREE**
- **Prior Train Status**: **Development Train 01 (`W0-A`, `B03`, `B10`, `B11`), Train 02 (`B20`, `B21`, `B22`), & Remediation (`R1–R7`) COMPLETE & COMMITTED (`c331dc3`, `0732ca0`)**
- **Current Phase**: **DEVELOPMENT TRAIN 03 COMPLETE — READY FOR GPT-6 REVIEW**
- **Baseline Git Commits**:
  - Base: `1ff5f04ed7a26969130eaba75da50b20684e763e` (`master`)
  - Commit 1 (`W0-A` Stabilization, 38 files): `e34516c`
  - Commit 2 (Architecture Baseline & Master Plan, 46 files): `2151f34`
  - Commit 3 (`W0–W2` Execution & Financial Correctness Baseline, 70 files): `c331dc3` (`c331dc3250937a664625b1ebb9e7fac6170b0ae3`)
  - Commit 4 (Persistent Project Execution Charter, 4 files): `0732ca0` (`0732ca07cd2cb3fcf6f45bef85d93f8fd8bdcb21`)
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

## Development Train 03 Batch Status (`Wave W3`: `B30 → B31 + B32` — `COMPLETE & VERIFIED IN WORKING TREE`)

### 1. Phase A (Train 03) — `B30` (`Frontend Replay State, TanStack Query Keys, Navigation Race Protection & Session Authority`)
- **Scope & Traceability**:
  - Close `GAP-FE-01`, `GAP-FE-02`, `GAP-FE-06`, and `GAP-FE-09` per `ADR-FE-001` (`docs/architecture/decisions/ADR-FE-001-state-ownership.md`) and `docs/plans/prompts/B30_fe-state.md`.
  - Created `frontend/src/features/replay/replayQueryKeys.ts` with canonical `replayQueryKeys` factory (`['replay', 'candles', sessionId, symbol, timeframe, adjustmentType, currentIndex]`, `['replay', 'indicators', sessionId, timeframe, adjustmentType, currentIndex, specHash]`, plus session/decisions/journal/practice/drawings keys).
  - Implemented `ReplayNavigationCoordinator` (`beginNavigation`, `isCurrent`, `abortActive`) with `AbortController` and monotonic `navigationToken` so rapid step/seek/timeframe actions abort in-flight queries and discard stale responses.
  - Implemented 4-step session cleanup (`cleanupSessionQueries`) in `useSessionSelection.ts` and `ReplayWorkspaceController.tsx` (`cancelQueries` → `removeQueries` → WebSocket teardown → single chart clear).
  - Implemented `dedupeCandlesByTimestamp(candles)` so duplicate WebSocket ticks + HTTP fetches at the same bar timestamp never duplicate candles.
  - Reduced `frontend/src/store/replayStore.ts` strictly to `{ sessionId, setSession, clearSession }`.
  - Configured explicit `QueryClient` defaults in `frontend/src/App.tsx` (`staleTime: 30_000`, `gcTime: 300_000`, `refetchOnWindowFocus: false`, `retry: 1`).
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**: `6/6 passed` in `frontend/src/features/replay/__tests__/b30_fe_state_race.test.tsx` (query key factory isolation, `×20` rapid navigation race with out-of-order latency, session switch cleanup, `replayStore` minimal surface, timestamp deduplication).
- **Internal QA Result**: `APPROVE`

---

### 2. Phase B (Train 03) — `B31` (`WebSocket Sequencing, Heartbeat, Reconnect & State Resynchronization`)
- **Scope & Traceability**:
  - Close `GAP-WS-01` and `GAP-FE-10` per `ADR-WS-001` (`docs/architecture/decisions/ADR-WS-001-ws-reliability.md`), `docs/testing/WEBSOCKET_TEST_MATRIX.md` (`WS-01..WS-10`), and `docs/plans/prompts/B31_ws.md`.
  - Upgraded `backend/app/api/ws_replay.py` with per-connection state (`ClientConnection`: `PAUSED | PLAYING | COMPLETED | ERROR`, independent `seq` counter per connection for `WS-10` multi-tab isolation), case-insensitive `_is_session_completed` fix (`SessionStatus.COMPLETED.value == "completed"`), strict JSON/action validation (`WS-03` returning `INVALID_JSON` / `UNKNOWN_ACTION` error frame without dropping connection), speed clamping (`[100, 4000]ms`, `WS-04`), dynamic `set_speed` (`WS-05`), `completed` frame on final bar (`WS-09`), `ping`/`pong` heartbeat, and `sync`/`resume` server resynchronization (`WS-01`, `WS-02`).
  - Preserved backward compatibility on `ConnectionManager` (`active_connections`, `playback_tasks`, `data` alias in `new_candle` frames) so all existing replay/scanner tests pass without modification.
  - Upgraded `frontend/src/hooks/useWebSocket.ts` with `WebSocketStatus` (`CONNECTING | OPEN | RECONNECTING | DISCONNECTED`), exponential backoff (`[500, 1000, 2000, 4000, 8000]ms`), `window` `online`/`offline` listeners (`WS-08`), heartbeat `ping` timer, monotonic `seq` deduplication (`seq <= lastSeqRef.current` dropped, `WS-07`), closed-socket command queueing + `onSendWhileClosed` callback (`WS-06`), and automatic `{ action: 'sync' }` upon reconnect.
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - Backend: `5/5 passed` in `backend/tests/test_b31_ws_replay.py` + `10/10 passed` in `backend/app/tests/test_ws_replay.py` & `test_scanner_replay_integrity.py`.
  - Frontend: `6/6 passed` in `frontend/src/hooks/__tests__/b31_ws_chaos_matrix.test.tsx`.
- **Internal QA Result**: `APPROVE`

---

### 3. Phase C (Train 03) — `B32` (`ChartKernel Facade, ReplayWorkspace Restructuring, Indicator Rendering Contract, Drawing & Chart Stability`)
- **Scope & Traceability**:
  - Close `GAP-FE-03`, `GAP-FE-04`, `GAP-FE-05`, `GAP-FE-07`, and `GAP-FE-08` per `ADR-CHART-001` (`docs/architecture/decisions/ADR-CHART-001-chart-kernel.md`) and `docs/plans/prompts/B32_chart.md`.
  - Created `frontend/src/components/chart/ChartKernel.ts` as the sole module calling `createChart` from `lightweight-charts` (`ChartKernel.createReplayKernel` and `ChartKernel.createEquityKernel`). Refactored `CandleChart.tsx` and `EquityChart.tsx` to use `ChartKernel` (`grep -rn "createChart(" frontend/src/` returns hits ONLY inside `ChartKernel.ts`).
  - Enhanced `IndicatorRenderRegistry.mapBackendDataWithResult` (`frontend/src/components/chart/IndicatorRenderRegistry.ts`) to detect `UNMAPPED_COLUMN` when `meta.status === 'OK'` but zero series map from non-empty backend rows, and wired `ReplayWorkspaceController.tsx`, `IndicatorManager.tsx`, `IndicatorPaneChrome.tsx`, and `ChartLegendOverlay.tsx` to display `warming (available/warmup)` and `UNMAPPED_COLUMN` / `INVALID_SPEC` badges.
  - Deleted the 3 dead legacy drawing files (`SumiDrawingAdapter.ts`, `DrawingToolRegistry.ts`, `WorkspacePersistence.ts`) and their test (`SumiDrawingAdapter.test.ts`), preserving `workspaceTypes.ts` and `SumiPrimitiveDrawingProvider.ts`.
  - Extracted `frontend/src/components/replay/ReplayHeader.tsx` and `frontend/src/components/replay/ReplayRightPanel.tsx` from `ReplayWorkspace.tsx` (reducing `ReplayWorkspace.tsx` from `765` lines to `297` lines) and gated hidden debug `<output>` DOM serialization behind `IS_DEV_OR_TEST`.
- **Implementation Status**: `COMPLETE_VERIFIED` (`UNCOMMITTED_IN_WORKTREE`)
- **Test Evidence**:
  - `3/3 passed` in `frontend/src/components/chart/__tests__/ChartKernel.test.tsx`
  - `6/6 passed` in `frontend/src/components/chart/__tests__/IndicatorContract.test.ts`
  - `9/9 passed` in `frontend/src/components/replay/__tests__/ReplayPage.test.tsx`
- **Internal QA Result**: `APPROVE`

---

### 4. Phase D (Train 03) — Full Multi-Train Verification & Product Smoke Gate Summary (`COMPLETE`)
- **Full Backend Pytest Suite (`PYTHONPATH=backend .venv/bin/pytest backend -q`)**: **`834 passed, 1 skipped, 0 failed`** (`835 collected`).
- **Full Frontend Vitest Suite (`npx vitest run`)**: **`253 passed, 0 failed`** across **`40/40` test files**.
- **Frontend Production Build (`npm run build`)**: **`0 TypeScript/Vite errors`**, built in `272ms`.
- **Live Browser Smoke Gate + Gate A1 Real WebSocket Reconnect (`Playwright Chromium`, `1440×1000` & `1280×800`)**: **`17/17 checks PASSED`**, **`0 consoleErrors`**, **`0 pageErrors`** (`artifacts/local/development-train-03/smoke/train03-smoke-results.json`, `replay-1440x1000.png`, `replay-1280x800.png`).
  - Gate A1 (`b31.ws-real-connection-termination-and-reconnect-gate`) verified live termination of active WebSocket connection (`connId=4`, `preDropSeqs=[1..7]`), establishment of new connection (`connId=5`, `postDropSeqs=[1..9]` accepted without discard), ordered flushing of queued commands (`sync`, `set_speed=100`, `start`, `pause`), and full server/UI cursor and candle resynchronization (`#49 / 349`, `49/49` unique ordered candles).
- **Production Database Checksum (`backend/sumi.db`)**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes, `0` byte delta).
- **Status**: **`TRAIN 03 COMPLETE & GATE A1 VERIFIED`**
