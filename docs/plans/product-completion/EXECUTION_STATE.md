# SUMI V3 — DEVELOPMENT EXECUTION STATE

- **Active Train**: **Development Train 01+02 Remediation Mission (`R1 → R2 → R3 → R4 → R5 → R6 → R7`)**
- **Prior Train Status**: **Development Train 01 (`W0-A Closure → B03 → B10 → B11`) & Train 02 (`B20 → B21 → B22`) COMPLETE & VERIFIED**
- **Current Phase**: **FULL REGRESSION CLOSED — READY FOR TRAIN 01+02 BASELINE ACCEPTANCE**
- **Baseline Git Commits**:
  - Base: `1ff5f04ed7a26969130eaba75da50b20684e763e` (`master`)
  - Commit 1 (`W0-A` Stabilization, 38 files): `e34516c`
  - Commit 2 (Architecture Baseline & Master Plan, 46 files): `2151f34`
  - Post-Commit 2 Commits: **0 (Strictly No Additional Commits — Train 01, Train 02, and Remediation R1–R7 Remain Uncommitted in Working Tree)**
- **Production DB Invariant (`backend/sumi.db`)**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes — verified bit-exact, `0` byte delta)

---

## Development Train 01 Summary (`COMPLETE`)

1. **Phase A — `W0-A` (`B00`, `B01`, `B02` + `DB-SAFETY`)**: `COMPLETE_COMMITTED` (`e34516c`, `2151f34`; `151/151` tests passed in `artifacts/local/development-train-01/w0a_140_regression.log`).
2. **Phase B — `B03` (`Schema, Migration Compatibility & Preview Isolation`)**: `COMPLETE_UNCOMMITTED` (`14/14` tests passed in `artifacts/local/development-train-01/b03_tests.log`).
3. **Phase C — `B10` (`Replay Ledger, Horizon & Eligibility Zero-Lookahead Lock`)**: `COMPLETE_UNCOMMITTED` (`8/8` tests passed in `artifacts/local/development-train-01/b10_tests.log`).
4. **Phase D — `B11` (`Indicator Forward Projection Clamping & Typed Metadata Contract`)**: `COMPLETE_UNCOMMITTED` (`9/9` tests passed in `artifacts/local/development-train-01/b11_tests.log`).

---

## Development Train 02 Batch Status (`Wave W2`: `B20 → B21 → B22` — `COMPLETE & VERIFIED`)

### 1. Phase A (Train 02) — `B20` (`Execution Kernel Parity: Next-Open Across All Paths & Fill-Estimate Sizing`)
- **Scope**:
  - Close `GAP-EXEC-01` and `GAP-EXEC-07` per `ADR-EXEC-001`, `DEC-03`, and `DEC-04`.
  - Unify Parameter Sweep (`backend/app/services/strategy_lab_service.py`) and Single Backtest (`backend/app/services/backtest_service.py`) onto `BacktestExecutionKernel` (`backend/app/domain/backtest/execution.py`) so `Signal[T] → Order QUEUED MARKET_NEXT_OPEN → Fill Open[T+1]` and `T = N - 1 → UNFILLED_AT_CUTOFF` (`G-01`, `G-03`, `G-05`, `G-12`).
  - Default `sl_tp_execution_mode` to `"next_open"` (`Open[T+2]` when bar $T+1$ breaches `SL`/`TP`, `G-04`) while supporting explicit `"intraday_touch"` mode when parameterized (`DEC-03`).
  - Implement causal `sizing_basis = "fill_estimate"` position sizing (`Open[T+1]` lot-100 calculation at fill time, provisional `Close[T]` estimate when queued at bar $T$, `G-11`, `DEC-04`).
  - Mark any retained legacy same-close helper/path with explicit `deprecated: True` metadata/banner.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`
- **Test Evidence**: `11/11 passed` in `backend/tests/test_b20_execution_kernel_parity.py` (`46/46` backtest suite; `G-01`, `G-03`, `G-04`, `G-05`, `G-11`, `G-12` verified).
- **Internal QA Result**: `APPROVE`
- **Open Issues**: None.
- **Dependencies**: `B01` (completed), `DEC-03` & `DEC-04` (approved), `DEC-TRAIN02-02` (ratified).
- **Next Action**: Advance to `B21` (completed).

---

### 2. Phase B (Train 02) — `B21` (`MarketProfile Single Source, T+1.5 Settlement & VN Calendar`)
- **Scope**:
  - Close `GAP-EXEC-02`, `GAP-EXEC-03`, `GAP-DB-08`, `GAP-DB-11`, and `GAP-U-01` (wording contract) per `ADR-SETTLE-001`.
  - Consolidate fee (`0.15%` buy/sell), sell tax (`0.10%`), board lot (`100`), default slippage (`0.0`), `settlement_mode` (`T+1.5_CALENDAR`), `settlement_label` (`T+1 · về sau 1 phiên`), and `sizing_basis` (`fill_estimate`) into a single authoritative `MarketProfile` in `backend/app/domain/market/`.
  - Wire `SettlementEngine` (`backend/app/domain/market/settlement.py`) and `TradingCalendar` (`backend/app/domain/market/calendar.py`, `VIETNAM_HOLIDAYS_V1`) across `execution.py`, `backtest_service.py`, `practice_workflow_service.py`, `trade_lifecycle_service.py`, `weekly_aggregator.py`, and `services/cafef/classifier.py` (`G-06`, `G-07`).
  - Use `math.isclose` float comparison in `classifier.py` (`GAP-DB-08`).
  - Standardize inline `T+1 · về sau 1 phiên` settlement wording in practice state / `TradeControls.tsx`.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`
- **Test Evidence**: `14/14 passed` in `backend/tests/test_b21_settlement_and_calendar.py`, `12/12 passed` in `test_b21_challenger_stress.py`, `28/28 passed` in `test_b21_challenger_contract.py`, `34/34 passed` in `test_b21_challenger_ingestion_and_float.py` (`G-06`, `G-07`, `GAP-DB-08`, `GAP-DB-11`, `GAP-U-01` verified).
- **Internal QA Result**: `APPROVE`
- **Open Issues**: None.
- **Dependencies**: `B20` (completed), `ADR-SETTLE-001`, `DEC-TRAIN02-03` (ratified).
- **Next Action**: Advance to `B22` (completed).

---

### 3. Phase C (Train 02) — `B22` (`Financial Metrics Unification: Net PnL, Win Rate, PF N/A, Sample Gate & Sharpe 252`)
- **Scope**:
  - Close `GAP-EXEC-04`, `GAP-EXEC-05`, and `GAP-EXEC-06` per `ADR-EXEC-001`, `TARGET_ARCHITECTURE_V3.md` §2, and `DEC-05`.
  - Unify realized PnL across `execution.py`, `metrics.py`, `batch_runner.py`, `analytics_service.py`, `strategy_lab_service.py`, `backtest_service.py`, `practice_workflow_service.py`, and `trade_lifecycle_service.py` to use `accounting.calculate_net_pnl` (`G-01`, `G-02`, `G-03`).
  - Unify winner rule to `net_pnl > 0` (`breakeven = not-winner`, `G-02`) and update `test_benchmark_metrics.py` (`TEST-MET-002`).
  - Unify Profit Factor on Net PnL (`sum(net_wins) / abs(sum(net_losses))`); when total loss is `0`, return `profit_factor = None` with `profit_factor_display = "N/A (chưa có lệnh thua)"` (`G-08`).
  - Enforce sample size warning gate `low_sample_warning = (total_closed_trades < 30)` with `"mẫu nhỏ (<30), độ tin cậy thấp"` (`G-09`).
  - Unify annualized daily return Sharpe (`rf = 0`, `ddof = 1`, `sqrt(252)`) and close-based Max Drawdown across `metrics.py` (`BenchmarkMetricRow`), `batch_runner.py`, and `analytics_service.py` (`G-10`).
- **Implementation Status**: `COMPLETE_UNCOMMITTED`
- **Test Evidence**: `10/10 passed` in `backend/tests/test_b22_metrics_unification.py`, `14/14 passed` in `test_b22_challenger_accounting_stress.py`, `11/11 passed` in `test_b22_challenger_parity_stress.py`, `11/11 passed` in `test_benchmark_metrics.py` (`G-01..G-03`, `G-08..G-10`, `DEC-05` verified).
- **Internal QA Result**: `APPROVE`
- **Open Issues**: None.
- **Dependencies**: `B21` (completed), `DEC-05` (approved), `DEC-TRAIN02-04` (ratified).
- **Next Action**: Full multi-train regression and victory closeout (`COMPLETE`).

---

### 4. Phase D (Train 02) — Full Multi-Train Regression & Governance Verification (`COMPLETE`)
- **Scope**: Multi-train regression across Train 01 and Train 02, database invariance check, git commit immutability check, frontend test suite, and frontend production build.
- **Verification Summary**:
  - Train 02 Golden Suites (`B20`, `B21`, `B22`, `BenchmarkMetrics`): `46/46 passed` (`0.52s`).
  - Train 01 Regression Locks (`B03`, `B10`, `B11`): `12/12 passed` (`3.48s`).
  - Full Backend Pytest Suite: `748 passed, 1 skipped, 0 failed` in `15.87s` (Full log captured in `artifacts/local/development-train-02/full_regression_train02.log`).
  - Frontend Vitest Suite: `240/240 passed` across `38/38` test files (`8.15s`).
  - Frontend Production Build: Clean build in `264ms` (`0 errors`).
  - Production Database Checksum: `backend/sumi.db` SHA-256 is `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes, `0` byte delta).
  - Git Commit State: `git rev-parse HEAD` is `2151f34d8dbcafca1d3cc78e944b1562201caa7a` (`0` commits).
  - Working Tree Integrity: All `26` uncommitted Train 01 files and all Train 02 implementation and test files preserved cleanly in working tree.
- **Status**: `COMPLETE & VERIFIED`

---

## Development Train 01+02 Remediation Summary (Findings R1–R6 + R7 Governance)

### 1. Requirement R1 — Safe Alembic Pre-Created Database Stamping
- **Assigned Worker**: Worker M1 (`teamwork_preview_worker`)
- **Files Modified**: `backend/alembic/env.py`
- **Tests Created**: `backend/tests/test_r1_alembic_stamping.py` (12 test cases)
- **Scope & Fix**:
  - Replaced heuristic `if "candles" in table_names:` check with rigorous schema validation against `REQUIRED_HISTORICAL_TABLES` (20 required tables) and `REQUIRED_HISTORICAL_COLUMNS` (`executions.execution_candle_index`, `executions.trade_id`, `replay_sessions.source_type`, `decisions.stop_loss`, `trades.planned_entry_price`).
  - Stamping revision `a2d639ecd5d5` is permitted if and only if all 20 historical tables and 5 critical columns are present.
  - Partial schemas fail closed with actionable `RuntimeError` detailing missing tables/columns.
  - Fresh empty databases and existing migrated schemas bypass stamping cleanly.
- **Verification Evidence**: `12/12 passed` in `test_r1_alembic_stamping.py`, `26/26 passed` including B03 migration suites.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 2. Requirement R2 — Authoritative Settlement Calendar Enforcement
- **Assigned Worker**: Worker M2 (`teamwork_preview_worker`)
- **Files Modified**:
  - `backend/app/domain/backtest/execution.py`
  - `backend/app/services/trade_lifecycle_service.py`
  - `backend/app/services/practice_workflow_service.py`
- **Tests Created**: `backend/tests/test_r2_settlement_calendar.py` (7 test cases)
- **Scope & Fix**:
  - Eradicated heuristic bar-count fallbacks (`holding_bars >= self.min_holding_bars`, `session.current_index >= exc_bar_idx + 2`, `exc_bar <= current_index - 2`) that bypassed `SettlementEngine` and `TradingCalendar`.
  - Enforced `SettlementEngine.is_sellable()` with proper session phase mapping (`"open"`, `"intraday"`, `"close"`) across Backtest Kernel, Trade Lifecycle, and Practice Workflow.
  - Enforced T+2 morning rejection versus permitted afternoon settlement phase, weekend non-trading day settlement, and multi-day holiday closures (Hùng Kings, Reunification Day, Labor Day, Tết).
- **Verification Evidence**: `7/7 passed` in `test_r2_settlement_calendar.py`, `21/21 passed` including `test_b21_settlement_and_calendar.py`, `39/39 passed` across backtest kernel suites.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 3. Requirement R3 — Authoritative Sharpe Risk-Free-Rate Coherence
- **Assigned Worker**: Worker M3 (`teamwork_preview_worker`)
- **Files Modified**:
  - `backend/app/domain/backtest/metrics.py`
  - `backend/app/services/analytics_service.py`
  - `backend/app/tests/test_analytics_trust.py`
- **Tests Created**: `backend/tests/test_r3_sharpe_coherence.py` (4 test cases)
- **Scope & Fix**:
  - Unified default risk-free rate to `rf_annual = 0.0` across `metrics.py` and `AnalyticsService._build_metric_results`, harmonizing API responses with Golden Case G-10 and DEC-05.
  - Unified daily compounding conversion: `((1.0 + rf_annual) ** (1.0 / 252.0) - 1.0) if rf_annual > 0 else 0.0`.
  - Preserved statistical presentation trust gating (`MIN_STATISTICAL_SAMPLE = 30`), returning `status="insufficient_data"` and `value=None` when sample $N < 30$ while allowing explicit `rf_annual` overrides.
- **Verification Evidence**: `4/4 passed` in `test_r3_sharpe_coherence.py` ($N=45$ bit-exact 4-decimal parity, $N=25$ trust gating, explicit $rf=0.045$, flat returns), `23/23 passed` including `test_analytics_trust.py` and `test_b22_metrics_unification.py`.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 4. Requirement R4 — Replay Cursor Future-State Isolation
- **Assigned Worker**: Worker M4 (`teamwork_preview_worker`)
- **Files Modified**: `backend/app/api/decisions.py`
- **Tests Created**: `backend/tests/test_r4_replay_future_state.py` (7 test cases)
- **Scope & Fix**:
  - Eliminated future state leakage at replay cursor $K$: orders created before or at $K$ and cancelled after $K$ return masked `status="pending"` until cursor advances to the cancellation bar.
  - Dynamically projected position state at cursor $K$ via `PracticeWorkflowService.get_snapshot` without mutating persisted SQLite rows, properly showing open positions when closed at a future cursor.
  - Enforced post-session audit eligibility: `include_future=True` rejected with HTTP 403 during active replay and permitted only for terminal/final-bar sessions.
- **Verification Evidence**: `7/7 passed` in `test_r4_replay_future_state.py`, `16/16 passed` including `test_b03_challenger_stress.py`, `30/30 passed` including `test_b10_replay_lookahead_lock.py` and `test_api_integration.py`.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 5. Requirement R5 — MarketProfile Fee, Tax, and Slippage Coherence
- **Assigned Worker**: Worker M5 (`teamwork_preview_worker`)
- **Files Modified**: `backend/app/domain/backtest/execution.py`
- **Tests Created**: `backend/tests/test_r5_market_profile_coherence.py` (8 test cases)
- **Scope & Fix**:
  - Changed `BacktestExecutionKernel.__init__` defaults to `None`, dynamically resolving rates from explicit `profile` when supplied (`fee_buy`, `tax_sell`, `slippage`, `settlement_days`, `sizing_basis`) while respecting explicit constructor overrides.
  - Dynamically resolved active rates in `run()` and applied them consistently across BUY cash deductions, SELL cash additions, order fill pricing (`open_price * (1 +- slippage)`), and `compute_round_trip_pnl(profile=active_profile)`.
  - Guaranteed mathematical accounting identity $\Delta\text{cash} \equiv \sum \text{net\_pnl}$ and exact trade net PnL agreement, preserving Golden Case G-01 default profile backward compatibility.
- **Verification Evidence**: `8/8 passed` in `test_r5_market_profile_coherence.py`, `19/19 passed` including `test_b20_execution_kernel_parity.py`, `54/54 passed` across backtest accounting suites.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 6. Requirement R6 — Import Preview → Accept Contract Integrity
- **Assigned Worker**: Worker M6 (`teamwork_preview_worker`)
- **Files Modified**:
  - `backend/app/api/import_data.py`
  - `backend/app/services/import_workflow_service.py`
  - `frontend/src/pages/ImportPage.tsx`
- **Tests Created**: `backend/tests/test_r6_import_contract.py` (5 test cases)
- **Scope & Fix**:
  - Changed `POST /api/import/preview` parameter default to `confirm: bool = Form(False)`, ensuring initial preview generates 0 database writes.
  - Added `staged: bool` field to `ImportPreviewResponse` schema and implementation.
  - Failed closed with HTTP 400 (`Cannot accept unstaged preview. Staged run required before acceptance.`) when attempting to accept an unstaged ephemeral run ID.
  - Aligned `frontend/src/pages/ImportPage.tsx` with a two-step flow: read-only preview by default, followed by staging and acceptance upon explicit user confirmation.
- **Verification Evidence**: `5/5 passed` in `test_r6_import_contract.py`, `17/17 passed` including `test_b03_alembic_and_preview_isolation.py` and `test_import_workflow.py`.
- **Implementation Status**: `COMPLETE_UNCOMMITTED`

---

### 7. Requirement R7 & Final Full-Regression Closure — Governance, Safety Invariants & Packaging
- **Files Modified (Test-Only Closure)**:
  - `backend/app/tests/test_backtest_kernel.py`
  - `backend/app/tests/test_trade_lifecycle.py`
  - `backend/tests/test_t2_settlement.py`
  - `backend/app/tests/test_import_api.py`
  - `docs/plans/product-completion/EXECUTION_STATE.md`
  - `docs/plans/product-completion/DECISION_LOG.md`
- **Artifacts Updated**: `artifacts/local/train_01_02_remediation/`
  - `full_backend_regression.log` (full 829-test backend suite log: 828 passed, 1 skipped, 0 failed)
  - `diff.patch` (clean unified diff of all remediation changes)
  - `test_summary.txt` (exact pytest and npm test logs)
  - `database_integrity.txt` (SHA256 checksum and size verification of `backend/sumi.db`)
  - `git_status.txt` (git status confirming working tree integrity)
  - `REMEDIATION_SUMMARY.md` (concise documentation of R1–R7 and final regression closure)
- **Verification Summary**:
  - Full Backend Pytest Suite (`PYTHONPATH=backend .venv/bin/pytest backend --import-mode=importlib`): **`828 passed, 1 skipped, 0 failed`** (`829 collected` in `22.55s`).
  - Focused R1–R6 + Adversarial Test Suite: **`78/78 passed, 0 failed`** (`43` R1–R6 + `35` adversarial challenger tests).
  - Core V3 & Golden regression suite: `281/281 passed` in 8.31s.
  - Frontend Vitest suite: `240/240 passed` across 38 test files in 7.67s.
  - Frontend Production Build: Clean build in 263ms (0 errors).
  - Production Database Checksum: `backend/sumi.db` SHA-256 is `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes, strictly untouched).
  - Git Commit State: `git rev-parse HEAD` is `2151f34d8dbcafca1d3cc78e944b1562201caa7a` (`0` new commits, 0 staging, 0 stash).
- **Status**: **`FULL REGRESSION CLOSED — READY FOR TRAIN 01+02 BASELINE ACCEPTANCE`**
