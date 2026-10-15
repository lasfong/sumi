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
