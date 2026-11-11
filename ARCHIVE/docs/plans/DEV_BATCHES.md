# DEV BATCHES — Sumi V3 (tiny, self-contained, STOP-gated)

**Luật chung mọi batch:** temp-DB only (cấm mutate `backend/sumi.db`); ExecPlan theo `PLANS.md` trước code; 1 capability/batch; không đổi contract ngoài scope (gặp → STOP); không workaround UX che domain (→ STOP); đỏ → STOP, không weaken test. Single-writer. Verify: `.\scripts\verify-v2.ps1` + UAT liên quan khi chạm Replay/Chart/Indicator/Drawing.

## B00 — DB path absolute + guard CWD
- **Đóng:** GAP-DB-01. **Cho phép:** `backend/app/config.py`, `backend/alembic/env.py`, startup assert, docs chạy từ `backend/`. **Cấm:** mọi service logic, migration data.
- **Pre:** backup `sumi.db` root 0B (giữ evidence). **Impl:** `DATABASE_URL` default absolute neo `Path(__file__)`; assert CWD/file không tạo DB lạ; alembic env cùng resolver.
- **Invariant:** §data provenance/DB path. **Tests:** khởi động từ `/tmp` không tạo file; UAT temp-DB hash prod unchanged. **Expected:** không còn file ma sau xóa backup. **Rollback:** revert 2 file. **DoD:** test xanh + doc cập nhật. **Reviewer:** check `Path(__file__)` + test CWD lạ. **STOP:** nếu tooling khác phụ thuộc relative path → dừng, ghi quyết định.

## B01 — Transaction + FK
- **Đóng:** GAP-DB-02/03. **Cho phép:** `dependencies.py`, `db.py`, write-paths bọc try/except-rollback (import/sync/weekly/rollback). **Cấm:** đổi schema, đổi logic derive.
- **Impl:** decorator/context `atomic()`; `get_db` rollback-on-error; `PRAGMA foreign_keys=ON` listener. **Tests:** inject lỗi giữa flush→commit vẫn atomic; FK reject/cascade drawing. **Rollback:** revert guard (không migration). **STOP:** lỗi lộ transaction kẹt → dừng.

## B02 — Synthetic deny + quarantine
- **Đóng:** GAP-DB-04. **Cho phép:** providers ssi/vnstock guard, `sync_workflow.execute`, banner FE, `Settings`.
- **Impl:** `is_synthetic` flag; default-deny trừ `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`; quarantine dataset. **Tests:** execute stub mặc định 403; bật cờ mới ghi quarantine. **STOP:** nếu sync_runs prod bị chạm → dừng.

## B03 — Alembic full models + guards + preview-confirm
- **Đóng:** GAP-DB-05/10/14. **Cho phép:** `alembic/env.py`, migration guard `_has_column`, `seed_cafef.py`/import preview tách commit. **Tests:** migrate từ head rỗng đủ 20 tables; preview thiếu confirm không commit.

## B10 — Replay ledger filter
- **Đóng:** GAP-REPLAY-01, GAP-EXEC-08. **Cho phép:** `api/decisions.py`, orders/trades routes, `practice-state` doc. **Impl:** `candle_index<=current_index` predicate hoặc deprecate trong replay; clamp `total_bars/release_date`. **Tests:** tại index N raw không trả activity >N.

## B11 — Projected drop server-side
- **Đóng:** GAP-REPLAY-02. **Cho phép:** `indicator_engine.py` union/reindex, `api/replay.py` indicators block, FE `MappingResult` type (chưa tách god).
- **Impl:** cắt `<=max_visible_ts` policy; FE phân biệt warming/mapping/server/projected/transport. **Tests:** Ichimoku +26 không payload/vẽ.

## B20 — Execution kernel parity (next-open)
- **Đóng:** GAP-EXEC-01/07. **Cho phép:** `domain/backtest/*`, `backtest_service.py`, `strategy_lab_service.py` sweep routing, lifecycle fill. **Cấm:** đổi fee/settlement (sang B21).
- **Impl:** sweep qua kernel + phases; sizing trên fill-estimate; legacy deprecated banner. **Tests:** parity sweep==batch; golden 6 cases. **STOP:** lệch PnL không giải thích được → dừng.

## B21 — MarketProfile + Settlement duy nhất
- **Đóng:** GAP-EXEC-02/03, GAP-U-01 (wording). **Cho phép:** `market/*`, `settlement.py`, `broker.py`, profile config, TradeControls inline text. **Tests:** settlement qua weekend/lễ; slippage single-source grep.

## B22 — Metrics unify (net/winner/PF/gate/Sharpe/MaxDD)
- **Đóng:** GAP-EXEC-04/05/06. **Cho phép:** `metrics.py`, `analytics_service.py`, batch runner, practice scoreboard. **Tests:** gross/net unify; breakeven=not-winner; PF-inf→N/A; gate N≥30 warn; Sharpe 1 ngữ nghĩa.

## B30 — FE keys + nav token + switch invalidate
- **Đóng:** GAP-FE-01/02/06. **Cho phép:** `Controller.tsx`, `useSessionSelection.ts`, `store/replayStore.ts` (thu sessionId), QueryClient defaults. **Cấm:** WS reconnect (sang B31), facade (sang B32).
- **Tests:** step-x20 race; switch no-flash; single-clear grep.

## B31 — WS reliability
- **Đóng:** GAP-WS-01, GAP-FE-10. **Cho phép:** `ws_replay.py`, `useWebSocket.ts`, Controller play/speed. **Tests:** chaos matrix (restart giữa play, offline, bad JSON, speed clamp).

## B32 — ChartKernel facade + god-split step 1 + drawing cleanup
- **Đóng:** GAP-FE-03/04/05/07. **Cho phép:** `ChartKernel/*`, CandleChart/Pane/Series/RenderRegistry `MappingResult`, xóa 3 file legacy, tách Header/Dock trước. **Tests:** grep createChart duy nhất; UAT pan/zoom/advance giữ drawings.

## B40 — Contracts snapshot + error envelope + dead routes
- **Đóng:** GAP-API-01/03/04/05/06/07/08/09/11 (part). **Cho phép:** `schemas/*`, FE `types/*`, `client.ts`, `main.py` handler, xóa catalog dead + importCafefData. **Tests:** openapi snapshot CI; 422 normalize UAT.

## B41 — BB envelope unify
- **Đóng:** GAP-API-02/11 (BB part). **Cho phép:** `domain/bb/*`, `api/bb.py`, `bbApi.ts`, `BBViewer.tsx`. **Tests:** snapshot 2 formats → 1; `horizons[H].bb_value` map; N/A fallback có lý do.

## B50 — T+2 inline + SL/TP/SHORT/sync + cancel LIMIT
- **Đóng:** GAP-U-01/02/04 (part). **Cho phép:** `TradeControls.tsx`, `PositionPanel.tsx`, `PendingOrders.tsx`. **Tests:** UAT matrix BUY→inline lý do; SL/TP invalid có alert; SHORT dấu đúng; sync mọi direction; hủy LIMIT.

## B51 — Builder→Battle + undo discoverability + orphan nav + assumptions
- **Đóng:** GAP-U-03/04/05, GAP-U-06..12 (gom). **Cho phép:** `StrategyRuleBuilder.tsx`, `StrategyLabPage.tsx`, `DrawingToolbar.tsx`, `KeyboardShortcutsModal.tsx`, `Sidebar.tsx`, `App.tsx` redirect, assumptions components. **Tests:** critical BUILDER→BATTLE prefill; undo shortcut+toast; `/backtest` redirect; assumptions mọi nơi.
