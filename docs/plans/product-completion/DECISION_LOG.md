# SUMI V3 — DEVELOPMENT TRAIN DECISION LOG

Concise record of architectural, governance, and commit-gate decisions across Development Trains 01 and 02.

---

## Ratified & Approved Governance Decisions

### `DEC-01` — W0-A Baseline Acceptance, ADR Addenda Ratification & Git Commit Authorization (`APPROVED`)
- **Date**: 2026-10-14
- **Phase**: Train 01 — Phase A (`B00`, `B01`, `B02`, `DB-SAFETY`)
- **Status**: **`APPROVED / RATIFIED`** (Authorized by Product Owner, 2026-10-14)
- **Outcome**:
  - Ratified `ADR-PROV-001-ADDENDUM-01`, `ADR-TX-001-ADDENDUM-01`, and `ADR-TX-001-ADDENDUM-02`.
  - Created Commit 1 (`e34516c`, 38 Group 1 `W0-A` files) and Commit 2 (`2151f34`, 46 Group 2 architecture/plan/testing/governance files).

---

### `DEC-02` — Acceptance of Development Train 01 Implementation (`B03`, `B10`, `B11`) & Working Tree Inspection Policy (`ACCEPTED / RECORDED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 01 Closeout (`B03`, `B10`, `B11`)
- **Status**: **`ACCEPTED / PRESERVED UNCOMMITTED IN WORKING TREE`**
- **Outcome**:
  - `B03`, `B10`, and `B11` completed and verified across isolated temporary SQLite databases (`568 passed` backend, `240 passed` frontend, `0` bytes delta on `backend/sumi.db`).
  - Per Product Owner instruction, all `B03`, `B10`, and `B11` changes remain uncommitted in the working tree while Development Train 02 (`B20 → B21 → B22`) builds forward on top of the current state without creating unauthorized Git commits.

---

### `DEC-03`, `DEC-04`, `DEC-05` — Development Train 02 (`Wave W2`: `B20 → B21 → B22`) Financial Semantics Authorization (`APPROVED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 02 (`B20`, `B21`, `B22`)
- **Status**: **`APPROVED / ACTIVE`** (Authorized by Product Owner per ratified `ADR-EXEC-001`, `ADR-SETTLE-001`, `TARGET_ARCHITECTURE_V3.md` §2, and `FINANCIAL_GOLDEN_CASES.md` `G-01..G-12`)
- **Approved Financial Decisions**:
  1. **`DEC-03` (Intraday SL/TP Fill Timing — `B20`, `G-04`)**:
     - Default `sl_tp_execution_mode` to `"next_open"` (`Open[T+2]` when bar $T+1$ breaches `SL`/`TP`) per `ADR-EXEC-001` and Golden Case `G-04`, while supporting an explicit documented `"intraday_touch"` mode flag when explicitly requested by legacy test configurations.
  2. **`DEC-04` (Causal Position Sizing on `fill_estimate` — `B20`, `G-11`)**:
     - Record `sizing_basis = "fill_estimate"` on orders; compute provisional quantity at bar $T$ close (`Close[T]`), and finalize board-lot (`100` shares) quantity at bar $T+1$ open (`Open[T+1]` fill price) using `floor(risk_cash / abs(fill_est - sl_price) / 100) * 100` capped by `max_affordable` at `Open[T+1]` (including `fee_buy`).
  3. **`DEC-05` (Net PnL, Breakeven Winner Rule & Zero-Loss Profit Factor — `B22`, `G-02`, `G-08`, `G-09`, `G-10`)**:
     - Enforce `net_pnl > 0` as the sole winner rule (`breakeven = not-winner`), compute Profit Factor on Net PnL (`sum(net_wins) / abs(sum(net_losses))`), return `profit_factor = None` with `profit_factor_display = "N/A (chưa có lệnh thua)"` when gross/net loss is `0`, flag `low_sample_warning = True` when `closed_trades < 30`, and unify annualized daily Sharpe (`rf = 0`, `ddof = 1`, `sqrt(252)`) across `metrics.py`, `batch_runner.py`, and `analytics_service.py`.

---

## Recorded Technical & Operational Decisions

### `DEC-TRAIN01-01` — Sequential Single-Writer Order (`B03 → B10 → B11`)
- **Status**: `COMPLETED`

### `DEC-TRAIN01-02` — Non-Destructive Additive Migration Strategy for `B03`
- **Status**: `COMPLETED`

### `DEC-TRAIN02-01` — Sequential Single-Writer Order & Working Tree Preservation (`B20 → B21 → B22`)
- **Date**: 2026-10-14
- **Status**: `COMPLETED`
- **Decision**:
  - Execute Development Train 02 strictly sequentially across `B20` (first) → `B21` (second) → `B22` (third).
  - Preserve all uncommitted Train 01 (`B03`, `B10`, `B11`) working-tree changes intact (never revert, reset, checkout, or stash them).
  - Do **NOT** create any Git commits during Train 02 (`HEAD` must remain at `2151f34`).
  - Keep `backend/sumi.db` bit-exact (`SHA-256: 234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`, `188,416` bytes, `0` bytes delta).

---

### `DEC-TRAIN02-02` — Batch B20 Acceptance: Execution Kernel Parity Ratified (`RATIFIED / ACCEPTED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 02 — Batch B20 (`Execution Kernel Parity`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified canonical `Open[T+1]` execution kernel routing across Single Backtest, Batch Battle, and Parameter Sweep (`Sweep == Batch == Single` numerical parity, closing `GAP-EXEC-01` and `GAP-EXEC-07`).
  - Standardized next-open fill semantics (`Signal[T] -> Order QUEUED MARKET_NEXT_OPEN -> Fill Open[T+1]`).
  - Enforced final bar boundary cutoff (`T = N - 1 -> UNFILLED_AT_CUTOFF`, `G-05`).
  - Established `sl_tp_execution_mode = "next_open"` as default (`Open[T+2]` on SL/TP breach at bar $T+1$, `G-04`) while maintaining `"intraday_touch"` backward compatibility.
  - Ratified causal `sizing_basis = "fill_estimate"` (`Close[T]` provisional estimate when queued, board-lot 100 `Open[T+1]` calculation at fill time, cash solvency guard, `G-11`).
  - Deprecated legacy same-close execution loops. Verified with 11/11 golden tests in `test_b20_execution_kernel_parity.py`.

---

### `DEC-TRAIN02-03` — Batch B21 Acceptance & Remediation: Single MarketProfile, SettlementEngine & Vietnam Calendar Ratified (`RATIFIED / ACCEPTED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 02 — Batch B21 (`MarketProfile, Settlement & Calendar`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified single authoritative `MarketProfile` in `backend/app/domain/market/rules.py` consolidating statutory Vietnam market constants (`lot_size = 100`, `fee_buy = 0.0015`, `fee_sell = 0.0015`, `tax_sell = 0.0010`, `slippage = 0.0`, `settlement_days = 2`, `settlement_mode = "T+1.5_CALENDAR"`, `settlement_label = "T+1 · về sau 1 phiên"`, `sizing_basis = "fill_estimate"`), eliminating duplicate definitions across 5 modules (`GAP-EXEC-02`).
  - Ratified `TradingCalendar` with complete 2018–2026 Vietnamese exchange holiday definitions (`VIETNAM_HOLIDAYS_V1`), properly treating compensatory weekend sessions as non-trading days.
  - Ratified `SettlementEngine` enforcing `T+1.5_CALENDAR` settlement cycle with FIFO lot release queues, weekend settlement (`G-06`), and multi-day holiday settlement (`G-07`).
  - Ratified strict anti-lookahead horizon clamping in `PracticeWorkflowService` (`earliest_release_date = None` when release date exceeds session candle horizon or `session.end_date`).
  - Ratified `SettlementLockDetail` dual-compatibility dict wrapper with bidirectional alias synchronization (`available_qty`/`available_quantity`, `blocked_qty`/`blocked_quantity`, `available_on`/`earliest_release_date`) and string-compatible membership (`"available_qty" in detail`).
  - Ratified holiday-aware weekly candle aggregation (`GAP-DB-11`) and IEEE-754 float tolerance (`math.isclose(rel_tol=1e-9, abs_tol=1e-6)`) in import classifier (`GAP-DB-08`).
  - Standardized UI settlement badge `"T+1 · về sau 1 phiên"` and lock tooltip in `TradeControls.tsx` (`GAP-U-01`). Verified with 88 tests across 4 test suites.

---

### `DEC-TRAIN02-04` — Batch B22 Acceptance: Metrics Engine Unification Ratified (`RATIFIED / ACCEPTED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 02 — Batch B22 (`Financial Metrics Unification`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified canonical accounting authority `compute_round_trip_pnl` and `RoundTripPnL` in `backend/app/domain/accounting.py` as single source of truth for realized PnL across all backend services (`GAP-EXEC-04`, `G-01..G-03`), supporting both profile types.
  - Ratified strict winner rule (`DEC-05`, `G-02`): winner strictly requires `net_pnl > 0.0`; breakeven (`math.isclose(net_pnl, 0.0, abs_tol=1e-9)`) is excluded from winner numerator and retained in total closed trades denominator (`win_rate = winning_trades / total_closed_trades`). Updated `test_benchmark_metrics.py` (`TEST-MET-002`).
  - Ratified Net Profit Factor (`DEC-05`, `G-08`): $\sum \text{net\_wins} / |\sum \text{net\_losses}|$. For zero-loss edge case: returns `profit_factor = None` (standard JSON `null`), and `profit_factor_display = "N/A (chưa có lệnh thua)"` when net wins > 0, or `"N/A"` on zero trades.
  - Ratified statistical sample size warning gate (`G-09`): flags `low_sample_warning = True` with `low_sample_message = "mẫu nhỏ (<30), độ tin cậy thấp"` when closed trades $N < 30$.
  - Ratified annualized daily return Sharpe ratio ($rf = 0.0$, sample std $\text{ddof} = 1$, $\sqrt{252}$, `0.0` when $N < 2$ or std $== 0$) and close-based Maximum Drawdown from daily close equity curve (`G-10`, `GAP-EXEC-06`), achieving bit-exact cross-module parity across `metrics.py`, `batch_runner.py`, `analytics_service.py`, `strategy_lab_service.py`, and `backtest_service.py`. Verified with 46 tests across 4 test suites.

---

### `DEC-TRAIN02-05` — Train 02 Victory & Regression Invariant Ratified (`RATIFIED / ACCEPTED`)
- **Date**: 2026-10-14
- **Phase**: Development Train 02 — Milestone 4 (`Closeout & Full Regression Gate`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Full multi-train regression successfully executed across Train 01 (`W0-A`, `B03`, `B10`, `B11`) and Train 02 (`B20`, `B21`, `B22`).
  - Backend pytest suite: **748 passed, 1 skipped, 0 failed** in 15.87s (100% pass rate).
  - Frontend Vitest suite: **240 passed, 0 failed** across 38 test files in 8.15s.
  - Frontend production build: Clean build in 264ms (0 errors).
  - Production database `backend/sumi.db`: Checksum bit-exact `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (188,416 bytes, 0 bytes delta).
  - Git commit immutability: Commit `HEAD` invariant at `2151f34d8dbcafca1d3cc78e944b1562201caa7a` (exactly 0 unauthorized git commits).
  - Working tree preservation: All 26 uncommitted files from Train 01 and all Train 02 implementation/test files preserved intact.

---

## Ratified Architectural Decisions — Train 01+02 Remediation Mission (R1–R7)

### `DEC-REM-01` — Alembic Safe Pre-Created Schema Validation (R1)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M1 (`Safe Alembic Stamping`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified strict validation of all 20 required historical application tables and 5 critical migration columns (`executions.execution_candle_index`, `executions.trade_id`, `replay_sessions.source_type`, `decisions.stop_loss`, `trades.planned_entry_price`) in `backend/alembic/env.py` before stamping revision `a2d639ecd5d5`.
  - Replaced single-table `if "candles" in table_names:` check. Partial schemas immediately fail closed with an actionable `RuntimeError` diagnostic detailing the exact missing tables or columns.
  - Fresh schemas and databases with existing `alembic_version` entries bypass stamping, ensuring regular migration flow is completely preserved without skipping or rewriting historical migration steps. Verified by 12 isolated regression tests in `backend/tests/test_r1_alembic_stamping.py`.

---

### `DEC-REM-02` — Authoritative Settlement Calendar Enforcement (R2)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M2 (`Settlement Calendar Enforcement`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified complete removal of all heuristic bar-count fallbacks (`holding_bars >= self.min_holding_bars`, `session.current_index >= exc_bar_idx + 2`, `exc_bar <= current_index - 2`) that previously bypassed official calendar rules.
  - Ratified `SettlementEngine.is_sellable()` as the sole authority across Backtest Kernel (`execution.py`), Trade Lifecycle (`trade_lifecycle_service.py`), and Practice Workflow (`practice_workflow_service.py`), with explicit session phase resolution (`"open"`, `"intraday"`, `"close"`).
  - Enforced statutory T+1.5 settlement rules: morning Open on Day T+2 is strictly locked; settlement occurs in the afternoon session; weekend non-trading days and gazetted exchange holidays (Hùng Kings, Reunification Day, Labor Day, Tết) are excluded from settlement elapsed days. Rejections return structured `SettlementLockDetail` with HTTP 400. Verified with 7 tests in `backend/tests/test_r2_settlement_calendar.py` and 21 tests in `test_b21_settlement_and_calendar.py`.

---

### `DEC-REM-03` — Sharpe Risk-Free-Rate Coherence & Trust Gating (R3)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M3 (`Sharpe Risk-Free-Rate Coherence`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified default annual risk-free rate unification to `rf_annual = 0.0` across `backend/app/domain/backtest/metrics.py` and `backend/app/services/analytics_service.py` (`_build_metric_results`), aligning live API responses with Golden Case G-10 and DEC-05.
  - Ratified unified daily compounding formula `((1.0 + rf_annual) ** (1.0 / 252.0) - 1.0) if rf_annual > 0 else 0.0` for non-zero risk-free rates across all callers.
  - Ratified statistical trust presentation gating (`MIN_STATISTICAL_SAMPLE = 30`): when closed trades $N < 30$, `AnalyticsService` reports `status="insufficient_data"` and `value=None` (JSON `null`), preventing false-precision display in the UI while core numerical routines compute raw floats for $N \ge 2$. Verified with 4 deterministic regression tests in `backend/tests/test_r3_sharpe_coherence.py` ($N \ge 40$ bit-exact 4-decimal parity) and 9 tests in `test_analytics_trust.py`.

---

### `DEC-REM-04` — Replay Cursor Future-State Isolation & Masking (R4)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M4 (`Replay Future-State Isolation`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified cursor-isolated future masking in `backend/app/api/decisions.py` for `/sessions/{id}/orders` and `/sessions/{id}/position` without mutating persisted historical database rows.
  - Orders created before or at cursor $K$ and cancelled at a future bar $> K$ are masked to `status = OrderStatus.PENDING`, `filled_price = None`, `filled_at = None`, revealing cancellation only when the cursor reaches the cancellation bar.
  - Position state at cursor $K$ is projected dynamically using `PracticeWorkflowService.get_snapshot` based exclusively on executions with `execution_candle_index <= K`, accurately reflecting open positions even if closed at a subsequent bar.
  - Enforced post-session audit eligibility: `include_future=True` is strictly forbidden (HTTP 403) during active replay and permitted only when the session reaches a terminal state (`COMPLETED`, `EXPIRED`, `BANKRUPT`) or the final bar. Verified with 7 tests in `backend/tests/test_r4_replay_future_state.py` and 16 tests in `test_b03_challenger_stress.py`.

---

### `DEC-REM-05` — MarketProfile Fee, Tax, and Slippage Coherence (R5)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M5 (`MarketProfile Coherence`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified dynamic rate resolution in `BacktestExecutionKernel`: constructor argument defaults are set to `None`, allowing caller-supplied `profile` parameters (`fee_buy`, `fee_sell`, `tax_sell`, `slippage`) to take full effect without being silently overridden by hardcoded defaults, while preserving explicit constructor overrides.
  - Applied resolved rates consistently across execution cash flows: BUY cash deductions (`cost_per_share = fill_price * (1 + fee_buy)`), SELL cash additions (`net = gross - fee_sell - tax_sell`), order fill pricing (`open_price * (1 +- slippage)`), intraday exits, and `compute_round_trip_pnl(profile=active_profile)`.
  - Guaranteed mathematical accounting identity $\Delta\text{cash} \equiv \sum_{t \in \text{trades}} t.\text{net\_pnl}$ and $t.\text{net\_pnl} \equiv \text{rt\_pnl}.\text{net\_pnl}$. Default profile behavior remains strictly bit-compatible with Golden Case G-01. Verified with 8 tests in `backend/tests/test_r5_market_profile_coherence.py` and 19 tests in `test_b20_execution_kernel_parity.py`.

---

### `DEC-REM-06` — Import Preview Read-Only Default with Consent Staging (R6)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M6 (`Import Contract Integrity`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Ratified read-only default on `POST /api/import/preview`: parameter `confirm: bool = Form(False)` ensures initial file upload, parsing, and inspection perform exactly 0 database writes.
  - Added explicit `staged: bool` field to `ImportPreviewResponse` schema (`False` for ephemeral read-only previews, `True` for committed staged runs).
  - Ratified closed failure contract on `POST /api/import/runs/{run_id}/accept`: attempting to accept an unstaged ephemeral run ID raises HTTP 400 with actionable diagnostic `"Cannot accept unstaged preview. Staged run required before acceptance."`.
  - Aligned frontend import contract in `frontend/src/pages/ImportPage.tsx` to execute a seamless two-step flow: inspect in read-only mode, stage manifest upon user consent, and accept into database, without introducing unnecessary external storage infrastructure. Verified with 5 tests in `backend/tests/test_r6_import_contract.py` and 17 tests across import suites.

---

### `DEC-REM-07` — Full Remediation Verification & Working Tree Invariant Ratified (R7)
- **Date**: 2026-10-15
- **Phase**: Train 01+02 Remediation — Milestone M7 (`Full Regression, Governance & Packaging`)
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Multi-milestone remediation verification completed across R1, R2, R3, R4, R5, R6, and R7.
  - Focused R1–R6 test suite + adversarial challenger suite: 78 passed, 0 failed.
  - Core V3 regression suite: 281 passed, 0 failed.
  - Frontend Vitest suite: 240 passed, 0 failed across 38 test files.
  - Frontend production build: clean build in 263ms with 0 errors.
  - Production database `backend/sumi.db`: checksum bit-exact `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (188,416 bytes, strictly untouched).
  - Git commit immutability: Commit `HEAD` invariant at `2151f34d8dbcafca1d3cc78e944b1562201caa7a` (0 new commits, 0 staging, 0 stash).
  - Assembled review bundle in `artifacts/local/train_01_02_remediation/`.

---

### `DEC-REM-08` — Final Full-Regression Closure: 8 Stale Test Fixtures Aligned with R2 & R6 (`RATIFIED / ACCEPTED`)
- **Date**: 2026-10-16
- **Phase**: Train 01+02 Remediation — Final Full-Regression Closure
- **Status**: **`RATIFIED / ACCEPTED`**
- **Decision & Outcome**:
  - Audited and resolved all 8 remaining legacy backend test failures without modifying any production code or weakening accepted financial (`ADR-EXEC-001`), settlement (`ADR-SETTLE-001`, `G-06`, `G-07`), or import-preview (`DEC-REM-06`) semantics:
    1. `backend/app/tests/test_backtest_kernel.py::test_simultaneous_sl_tp_precedence` (`STALE_TEST_FIXTURE`): Replaced naive `2024-01-01 + timedelta(days=i)` dates (which included New Year holiday `2024-01-01` and Saturday `2024-01-06`) with official trading weekdays (`2024-01-08` to `2024-01-16`) and placed the `intraday_touch` wild bar on Day T+3 when conservative 1D settlement unlocks intraday exits.
    2. `backend/app/tests/test_trade_lifecycle.py::test_partial_reduce_keeps_trade_open_and_updates_cash` (`STALE_TEST_FIXTURE`): Updated start date from Friday `2024-03-01` (which stepped onto Saturday `2024-03-02` and Sunday `2024-03-03`) to Monday `2024-03-04` through Thursday `2024-03-07`.
    3. `backend/app/tests/test_trade_lifecycle.py::test_multiple_round_trips_same_session_symbol_have_independent_net_pnl` (`STALE_TEST_FIXTURE`): Replaced `2024-02-01 + timedelta(days=i)` (which placed bars on Saturday `2024-02-03` and Sunday `2024-02-04`) with 7 official trading weekdays (`2024-03-04` to `2024-03-12`).
    4. `backend/app/tests/test_trade_lifecycle.py::test_t2_settlement_detailed_rejection_feedback` (`STALE_TEST_FIXTURE` / `STALE_TEST_EXPECTATION`): Replaced Saturday `2024-06-01` start date with Monday `2024-06-03` through Friday `2024-06-07`, verifying T+1 rejection feedback reports Wednesday `2024-06-05` (`T+2`) as the earliest release date.
    5. `backend/tests/test_t2_settlement.py::test_market_buy_sell_accepted` (`STALE_TEST_FIXTURE`): Replaced non-deterministic `datetime.now() - timedelta(days=30)` + `timedelta(days=i)` (which placed bars 10–12 across Friday–Sunday) with deterministic `TradingCalendar` trading dates starting Monday `2024-03-04`.
    6. `backend/app/tests/test_import_api.py::test_import_accept_api` (`STALE_TEST_FIXTURE` / `STALE_TEST_EXPECTATION`): Updated to exercise read-only preview (`confirm=False` default, `staged=False`) → explicit staging (`confirm="true"`, `staged=True`) → accept (`200 OK`).
    7. `backend/app/tests/test_import_api.py::test_import_rollback_api` (`STALE_TEST_FIXTURE` / `STALE_TEST_EXPECTATION`): Updated to exercise read-only preview → explicit staging (`confirm="true"`) → accept → rollback (`200 OK`).
    8. `backend/app/tests/test_import_api.py::test_stale_preview_accept_api_contract` (`STALE_TEST_FIXTURE` / `STALE_TEST_EXPECTATION`): Updated both competing previews to stage with `confirm="true"` before testing intervening acceptance and stale-preview rejection (`400 Bad Request`, `"Bản xem trước đã hết hạn"`).
  - Full repository backend pytest suite (`PYTHONPATH=backend .venv/bin/pytest backend --import-mode=importlib`): **829 collected — 828 passed, 1 skipped, 0 failed** in 22.55s.
  - Focused R1–R6 + adversarial regression suites: **78/78 passed, 0 failed**.
  - Production database `backend/sumi.db`: SHA-256 `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes, 0 byte delta).
  - Status: `FULL REGRESSION CLOSED — READY FOR TRAIN 01+02 BASELINE ACCEPTANCE`.

---

### `DEC-06` — W0–W2 Baseline Closure Commit (`c331dc3`), Persistent Project Execution Charter (`0732ca0`) & Train 03 Authorization (`APPROVED`)
- **Date**: 2026-10-16
- **Phase**: W0–W2 Baseline Closure & Development Train 03 (`B30`, `B31`, `B32`)
- **Status**: **`APPROVED / RATIFIED`** (Authorized by Product Owner, 2026-10-16)
- **Outcome**:
  - Committed the complete `W0–W2` technical and financial baseline (`B03`, `B10`, `B11`, `B20`, `B21`, `B22`, remediation `R1–R6`, and final 8 legacy test fixture alignments — 70 files) as commit `c331dc3250937a664625b1ebb9e7fac6170b0ae3` (`feat(v3): complete W0-W2 execution and financial correctness baseline`).
  - Established the persistent Sumi V3 Project Execution Charter in `AGENTS.md` linking canonical governance records (`SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`, `EXECUTION_STATE.md`, `DECISION_LOG.md`, `FINANCIAL_GOLDEN_CASES.md`, and ratified ADRs) and reconciled `W0–W2` statuses on the Batch Board as commit `0732ca07cd2cb3fcf6f45bef85d93f8fd8bdcb21` (`docs(governance): establish Sumi V3 execution charter`).
  - Authorized Development Train 03 (`B30` first, followed by `B31` and `B32` with strict file-ownership discipline), keeping Train 03 implementation uncommitted until separate Product Owner authorization.

---

### `DEC-TRAIN03-01` — B30 Replay Query Keys, Navigation Token & Session Switch Cleanup (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 03 — Batch `B30` (`ADR-FE-001`)
- **Status**: **`APPROVED & COMMITTED (`12e166a`)`**
- **Decision & Outcome**:
  - Implemented `replayQueryKeys` in `frontend/src/features/replay/replayQueryKeys.ts` keying candle queries by `['replay', 'candles', sessionId, symbol, timeframe, adjustmentType, currentIndex]` and indicator queries by `['replay', 'indicators', sessionId, timeframe, adjustmentType, currentIndex, specHash]`.
  - Implemented `ReplayNavigationCoordinator` (`AbortController` + monotonic `navigationToken`) and `cleanupSessionQueries` (`cancelQueries` → `removeQueries` → WebSocket teardown → single chart clear) to prevent out-of-order HTTP responses or stale session/timeframe caches from flashing on screen.
  - Reduced `frontend/src/store/replayStore.ts` strictly to `{ sessionId, setSession, clearSession }` and configured explicit `QueryClient` defaults in `frontend/src/App.tsx`. Verified by `6/6` tests in `frontend/src/features/replay/__tests__/b30_fe_state_race.test.tsx`.

---

### `DEC-TRAIN03-02` — B31 WebSocket State Machine, Monotonic Sequence Envelope & Auto-Reconnect (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 03 — Batch `B31` (`ADR-WS-001`)
- **Status**: **`APPROVED & COMMITTED (`12e166a`)`**
- **Decision & Outcome**:
  - Upgraded `backend/app/api/ws_replay.py` with per-connection `ClientConnection` state (`PAUSED | PLAYING | COMPLETED | ERROR`), independent monotonic `seq` counters per connection (`WS-10` multi-tab isolation), case-insensitive `_is_session_completed` check (`SessionStatus.COMPLETED.value == "completed"`), strict JSON/action validation (`INVALID_JSON`, `UNKNOWN_ACTION` error frames without dropping the socket, `WS-03`), speed clamping (`[100, 4000]ms`, `WS-04`), dynamic `set_speed` (`WS-05`), `completed` frame (`WS-09`), `ping`/`pong`, and `sync`/`resume` resynchronization (`WS-01`, `WS-02`), while preserving legacy `ConnectionManager.active_connections` and `data` envelope fields for backward compatibility.
  - Upgraded `frontend/src/hooks/useWebSocket.ts` with `WebSocketStatus` (`CONNECTING | OPEN | RECONNECTING | DISCONNECTED`), exponential backoff (`[500, 1000, 2000, 4000, 8000]ms`), `online`/`offline` listeners (`WS-08`), heartbeat `ping` timer, monotonic `seq` deduplication (`WS-07`) with `lastSeqRef.current = 0` reset on every new socket `onopen`, closed-socket command queueing (`WS-06`) flushing `{ action: 'sync' }` first before queued commands, and live browser reconnect verification (`Gate A1`). Verified by `6/6` backend tests in `backend/tests/test_b31_ws_replay.py`, `7/7` frontend chaos tests in `frontend/src/hooks/__tests__/b31_ws_chaos_matrix.test.tsx`, and `17/17` live browser smoke checks (`b31.ws-real-connection-termination-and-reconnect-gate`).

---

### `DEC-TRAIN03-03` — B32 ChartKernel Facade, Indicator MappingResult UI & ReplayWorkspace Decomposition (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 03 — Batch `B32` (`ADR-CHART-001`)
- **Status**: **`APPROVED & COMMITTED (`12e166a`)`**
- **Decision & Outcome**:
  - Created `frontend/src/components/chart/ChartKernel.ts` as the sole module calling `createChart` from `lightweight-charts` (`ChartKernel.createReplayKernel` and `ChartKernel.createEquityKernel`), refactoring `CandleChart.tsx` and `EquityChart.tsx` accordingly.
  - Enhanced `IndicatorRenderRegistry.mapBackendDataWithResult` (`frontend/src/components/chart/IndicatorRenderRegistry.ts`) to return `status: 'UNMAPPED_COLUMN'` when `meta.status === 'OK'` but zero series map from non-empty backend rows, and wired `ReplayWorkspaceController.tsx`, `IndicatorManager.tsx`, `IndicatorPaneChrome.tsx`, and `ChartLegendOverlay.tsx` to display `warming (available/warmup)` and `UNMAPPED_COLUMN` / `INVALID_SPEC` badges.
  - Deleted the 3 dead legacy drawing files (`SumiDrawingAdapter.ts`, `DrawingToolRegistry.ts`, `WorkspacePersistence.ts`) and their test (`SumiDrawingAdapter.test.ts`), preserving `workspaceTypes.ts` and `SumiPrimitiveDrawingProvider.ts`.
  - Extracted `ReplayHeader.tsx` and `ReplayRightPanel.tsx` from `ReplayWorkspace.tsx` (reducing `ReplayWorkspace.tsx` from `765` lines to `297` lines) and gated hidden debug `<output>` DOM serialization behind `IS_DEV_OR_TEST` (`GAP-FE-08`). Verified by `ChartKernel.test.tsx` (`3/3`), `IndicatorContract.test.ts` (`6/6`), and `ReplayPage.test.tsx` (`9/9`).

---

### `DEC-07` — W3 Baseline Closure Commit (`12e166a`) & Development Train 04 (`W4`: `B40`, `B41`, `B50`, `B51`) Authorization (`APPROVED`)
- **Date**: 2026-10-16
- **Phase**: W3 Baseline Closure (`Gate A1` + `Gate A2`) & Development Train 04 (`W4`)
- **Status**: **`APPROVED / RATIFIED`** (Authorized by Product Owner, 2026-10-16)
- **Outcome**:
  - Verified live WebSocket connection termination and new-connection reconnect during active Replay playback (`Gate A1`, `b31.ws-real-connection-termination-and-reconnect-gate`).
  - Reconciled the exact 31-path `W3` inventory (`19` modified tracked paths, `4` deleted legacy drawing paths, `8` untracked source/test paths) and committed as `12e166ad323437b43607c6ba739f4b273a7a4d21` (`feat(v3): complete W3 replay state websocket and chart integration`).
  - Authorized Development Train 04 (`B40 → B41`, `B50 → B51`) in the working tree without creating additional commits until subsequent Product Owner review.

---

### `DEC-TRAIN04-01` — B40 API Contract Alignment, Error Envelope & Dead Endpoint Disposition (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 04 — Batch `B40` (`ADR-API-001`)
- **Status**: **`APPROVED & COMMITTED (`40f6563`)`**
- **Decision & Outcome**:
  - Standardized FastAPI global exception handlers in `backend/app/main.py` (`RequestValidationError`, `HTTPException`, `Exception`) emitting `{code, message, field_errors, detail}` and stripped raw `str(e)` exception leaks across API routers (`analytics.py`, `universe.py`, `indicators.py`), while adding `extractErrorMessage` in `frontend/src/api/client.ts` so HTTP 422 arrays and object details never render `[object Object]`.
  - Separated `CreateSessionMode` (`normal | random | blind_symbol | blind_date`) from `SessionMode` (which adds `backtest` for read/response DTOs) in `backend/app/schemas/replay_schema.py` and `frontend/src/types/replay.ts` (`GAP-API-03`), guarding `replayApi.createSession` on the client and preserving HTTP 422 rejection when `mode="backtest"` is sent to `POST /api/replay/sessions`.
  - Aligned `SessionStatus` (`'bankrupt'`), `DecisionCreate` validator (`OrderType.CUSTOM_PRICE`), `TradePlanResult` (all 7 risk fields), `AnalyticsReport` optionality, `checklist_snapshot` (`Record<string, unknown> | string | null` dual-read/write), `Candle.source: string | null`, and `/api/scanner/*` `response_model` annotations (`ScannerRunResponse`, `ScannerHistoryItemResponse`).
  - Enforced signal contract invariants (`1..2` signals max, unique names, non-null `params`, pinned `version`, no extra fields) in `frontend/src/api/signalsApi.ts`, `SignalExplanationInspector.tsx`, and `SignalInspector.tsx`.
  - Removed duplicate unannotated `GET /api/import/catalog` in `backend/app/api/import_data.py` (retaining canonical `GET /api/data/catalog` in `backend/app/api/symbols.py`), removed dead `importCafefData` in `frontend/src/api/importApi.ts`, and isolated legacy `DataFeedService` in `backend/app/services/data_service.py` (`GAP-API-08`, `GAP-API-10`, `GAP-DB-13`). Verified by `5/5` backend tests (`11` sub-assertions) in `test_b40_openapi_contract_snapshot.py` and `17/17` frontend tests.

---

### `DEC-TRAIN04-02` — B41 Money Flow BB Envelope Unification & Degraded/Warming State Contract (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 04 — Batch `B41` (`ADR-API-001`)
- **Status**: **`APPROVED & COMMITTED (`40f6563`)`**
- **Decision & Outcome**:
  - Added explicit Pydantic `response_model` declarations (`BBHorizonsResponseSchema`, `Union[BBSymbolResponseSchema, BBChartFormatResponseSchema]`, `MarketBBResponseSchema`) to all `/api/bb/*` GET endpoints in `backend/app/api/bb.py` and `backend/app/schemas/bb_schema.py` (`GAP-API-11`).
  - Unified `series` (`format=series`) and `chart` (`format=chart`) response contracts in `frontend/src/api/bbApi.ts` and `frontend/src/components/chart/TechnicalFlowBBViewer.tsx`, surfacing explicit warming (`is_warmup`), degraded (`UNAVAILABLE_DEGRADED` / low data quality / breadth `< 50%`), invalid, and empty states while rendering valid BB outputs through `ChartKernel` (`GAP-API-02`). Verified by `32/32` backend tests and `5/5` frontend tests.

---

### `DEC-TRAIN04-03` — B50 Trading Practice UX: Inline T+1.5 Feedback, SL/TP Geometry Alert, Symmetric SHORT R:R & Pending Limit Cancel (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 04 — Batch `B50` (`ADR-SETTLE-001`, `ADR-EXEC-001`)
- **Status**: **`APPROVED & COMMITTED (`40f6563`)`**
- **Decision & Outcome**:
  - Displayed `Khả dụng` vs `Chờ về` share columns in `PositionPanel.tsx` and an always-visible inline `T+1 · về sau 1 phiên` settlement notice (`data-testid="settlement-inline-notice"`) below `SELL`/`CLOSE` in `TradeControls.tsx`, plus distinct tooltips for `SELL` vs `CLOSE` (`GAP-U-01`, `GAP-U-12`).
  - Added inline Vietnamese SL/TP geometry validation (`data-testid="sl-tp-validation-alert"`) in `TradeControls.tsx` and `backend/app/domain/trade_planning.py` (`BUY`: `SL < Entry < TP`; `SELL`/`SHORT`: `TP < Entry < SL`) keeping the sizing preview visible while blocking invalid order submission, and made `SHORT` PnL, R-multiple, and bidirectional `Risk-Reward` chart drawing sync symmetric (`GAP-U-02`).
  - Implemented `DELETE /api/replay/sessions/{session_id}/orders/{order_id}` (`cancel_pending_order`) in `backend/app/services/trade_lifecycle_service.py` and `backend/app/api/decisions.py` and wired a visible `Hủy lệnh` button in `PendingOrdersPanel.tsx` (`GAP-U-04`).
  - Added `MarketProfile` assumptions (`Phí mua 0.15% · Phí bán 0.15% · Thuế bán 0.10% · Lô chẵn 100 CP · T+1 · về sau 1 phiên`) inside the `TradeControls.tsx` order dialog (`GAP-U-11`). Verified by `2/2` backend tests (`4` sub-assertions) in `test_b50_trading_practice_ux.py`, `4/4` frontend tests in `b50_trading_practice_ux.test.tsx`, and live browser smoke checks.

---

### `DEC-TRAIN04-04` — B51 Strategy Lab Single Tab Integration, Rule Builder Handoff & 1280px Desktop Layout (`APPROVED / COMMITTED`)
- **Date**: 2026-10-16
- **Phase**: Development Train 04 — Batch `B51`
- **Status**: **`APPROVED & COMMITTED (`40f6563`)`**
- **Decision & Outcome**:
  - Integrated `BacktestPage` as the `Kiểm Định Đơn Lẻ (Single Backtest)` tab (`?tab=single`) inside `StrategyLabPage.tsx` with a `Single Backtest vs. Strategy Battle` guidance banner (`data-testid="single-vs-battle-banner"`), and redirected `/backtest` in `App.tsx` to `/strategy-lab?tab=single` (`GAP-U-05`).
  - Completed lossless `StrategyRuleBuilder` → `StrategyLabPage` (`Single` and `Battle`) handoff via `sumi_pending_strategy_rule` (`sessionStorage`), adding custom AST rule execution (`custom_ast_rule.yaml`) to `BacktestPage.tsx`, a prefill banner (`data-testid="custom-rule-prefill-banner"`), and toast feedback (`GAP-U-03`).
  - Upgraded Undo/Redo contrast and shortcut hints (`Ctrl+Z`/`Ctrl+Y`) in `DrawingToolbar.tsx`, `KeyboardShortcutsModal.tsx`, and `ReplayWorkspaceController.tsx` (with Spacebar interactive focus guard, `GAP-U-04`, `GAP-U-13`).
  - Updated `Sidebar.tsx` active tab matching (`GAP-U-06`), added visible `↑`/`↓` indicator order controls and warming badges in `IndicatorPaneChrome.tsx` & `ChartLegendOverlay.tsx` (`GAP-U-07`, `GAP-U-09`), extracted `Signals` into a 5th `PracticeRail.tsx` tab so `BUY`/`SELL` remain above the fold at `1440×1000` and `1280×800` (`GAP-U-08`), added contextual `📘 Nhật ký` / `📊 Phân tích` links in `ReplayRightPanel.tsx` (`GAP-U-12`), and standardized `< 1280px` desktop warnings (`xl:hidden`) and `MarketProfile` assumptions across pages (`GAP-U-10`, `GAP-U-11`). Verified by `2/2` backend tests in `test_b51_strategy_rule_builder_handoff.py`, `28/28` frontend tests, and live Playwright browser smoke at `1440×1000` and `1280×800`.

---

### `DEC-08` — W4 Baseline Closure Commit (`40f6563`) & Development Train 05 (`W5-UAT`) Full Product Acceptance Verification (`APPROVED / COMMITTED`)
- **Date**: 2026-10-17
- **Phase**: W4 Controlled Baseline Closure (`Phase A`) & Development Train 05 Full Product UAT (`Phase B–D`)
- **Status**: **`APPROVED & COMMITTED`**
- **Decision & Outcome**:
  - Reconciled backend pytest collection counts between `pytest backend --import-mode=importlib` (`844` items + `1` skipped module; `W3=834`, `W4=844`) and `pytest backend/tests backend/app/tests --import-mode=importlib` (`843` items + `1` skipped module; `W3=833`, `W4=843`, difference = `backend/scripts/test_connectors.py::test_feeds`). Verified that `W4` added `10` new backend pytest functions across 4 files and `15` new frontend Vitest tests (`253 → 268`).
  - Committed the verified 63-path `W4` (`B40`, `B41`, `B50`, `B51`) inventory as `40f6563ecc6ef4ce0c37ace90626e3380074c75c` (`feat(v3): complete W4 API contracts trading practice and strategy UX`).
  - Executed the 5 authoritative user journeys (`Journey 1: CafeF Import → Chart → Indicators`, `Journey 2: Replay → Drawing → Timeframe → Resume`, `Journey 3: Strategy Builder → Backtest → Compare`, `Journey 4: Scanner → Signal Explanation → Replay`, `Journey 5: Trading Practice → Journal → Analytics`) and Cross-Journey Quality Gates (`1440×1000`, `1280×800`, and `< 1280px`) on an isolated temporary SQLite database (`./artifacts/local/development-train-05/run_train05_uat.sh`).
  - Discovered and resolved 5 defects (`DEF-W5-01..05`) within accepted product scope without altering ratified financial/settlement formulas or touching `backend/sumi.db`:
    1. `DEF-W5-01` (`StrategyLabPage.tsx`): Added fallback to `response.runs[].analytics.equity_curve` so `MultiStrategyEquityChart` renders on multi-symbol Strategy Battle runs.
    2. `DEF-W5-02` (`backend/app/domain/bb/calculator.py`): Normalized `session_date` and `as_of` to ISO strings in `ProxyBBCalculator.calculate_series` to prevent `TypeError` when `as_of` is supplied.
    3. `DEF-W5-03` (`AnalyticsPage.tsx`, `BacktestPage.tsx`): Surfaced canonical Golden Case `G-08` (`"N/A (chưa có lệnh thua)"` + tooltip) and `G-09` (`"mẫu nhỏ (<30), độ tin cậy thấp"`) banners alongside `MarketProfile` assumptions.
    4. `DEF-W5-04` (`SessionDebriefModal.tsx`, `ScannerSourceContext.tsx`, `ReplayRightPanel.tsx`, `App.tsx`): Unified `SessionDebriefModal` win/loss/PF display with `AnalyticsService`, added reverse navigation links to `/scanner` and `/strategy-lab?tab=flow`, and rendered `.limited-workstation-warning` on non-`/replay` routes when viewport `< 1280px`.
    5. `DEF-W5-05` (`backend/app/services/trade_lifecycle_service.py`): Populated `dummy_decision` from the originating `Decision` record in `TradeLifecycleService.execute_pending_order` so `Trade` rows created from pending `MARKET_NEXT_OPEN` or `LIMIT` orders retain `setup_type`, `market_regime`, `emotion`, `stop_loss`, `target_price`, `planned_entry_price`, and `planned_r`.
  - Achieved **`101/101` UAT checks PASSING**, **`0` console/page errors**, **`0` external network requests**, **`844 passed, 1 skipped` backend tests**, **`268 passed` frontend tests**, clean production build, and bit-exact `backend/sumi.db` checksum (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`).

---

### `DEC-09` — W5 Technical Baseline Closure Commit & Owner Acceptance Handoff (`APPROVED`)
- **Date**: 2026-10-17
- **Phase**: Wave `W5` Technical Baseline Closure (`DEF-W5-01..05` + Governance Records)
- **Status**: **`APPROVED / RATIFIED`** (Authorized by Product Owner, 2026-10-17)
- **Decision & Outcome**:
  - Reconciled the W5 report's "13 modified files" header label against the actual **14 modified tracked files** (`11` source/test files: `backend/app/domain/bb/calculator.py`, `backend/app/services/trade_lifecycle_service.py`, `backend/tests/test_b41_bb_contract.py`, `backend/tests/test_b50_trading_practice_ux.py`, `frontend/src/App.tsx`, `frontend/src/components/replay/ReplayRightPanel.tsx`, `frontend/src/components/replay/ScannerSourceContext.tsx`, `frontend/src/components/replay/SessionDebriefModal.tsx`, `frontend/src/pages/AnalyticsPage.tsx`, `frontend/src/pages/BacktestPage.tsx`, `frontend/src/pages/StrategyLabPage.tsx` + `3` governance files: `DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`).
  - Authorized one controlled commit covering the exact 14 verified W5 paths with message `fix(v3): complete W5 product UAT remediation and governance records`, keeping local UAT artifacts (`artifacts/local/development-train-05/`) untracked and unpushed.
  - Enforced strict distinction between **`AUTOMATED_UAT_PASS`** (achieved across all 16 batches `B00–B51`, `844` backend tests, `268` frontend tests, `G-01..G-12` Golden Cases, and `101/101` Playwright UAT checks) and **`OWNER_ACCEPTED`** (`ACCEPTED_V3_COMPLETE`), which remains strictly reserved for final Product Owner sign-off using `artifacts/local/development-train-05/OWNER_UAT_GUIDE.md`.
