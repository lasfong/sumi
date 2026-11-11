# SUMI V3 — REPOSITORY CONTEXT INDEX

- **Document ID**: `SUMI-V3-CONTEXT-INDEX-V2`
- **Last Verified**: `2026-10-19` (Session `CTX-HARDEN-002`)
- **Repository Root**: `/Users/mizuhara/workspace/sumi`

> [!IMPORTANT]
> **Progressive Context Loading & Authority Rules**:
> 1. **Lookup Map, Not a Mandatory Reading List**: Do **not** indiscriminately read every file in this index. In **Phase A (Minimal Bootstrap)**, read only `AGENTS.md`, `SESSION_PROTOCOL.md`, `PROJECT_SNAPSHOT.md`, and your assigned entry in `TASK_REGISTRY.md`. Use this index in **Phase B** solely to locate the specific files required by your task.
> 2. **Appearance Does Not Confer Authority**: Do **not** assume a document is authoritative merely because it appears in this index. Check the `Authority Level` and `Loading Phase` columns below and follow the **6-Rank Authority Hierarchy** in [`SESSION_PROTOCOL.md`](SESSION_PROTOCOL.md) §1.1.
> 3. **No Silent Promotion**: Never promote `PROPOSED / UNRATIFIED` drafts or `HISTORICAL EVIDENCE` claims into `RATIFIED DECISION` project policy.
> 4. **Always Reconcile Against Live Git & DB State**: The 3 pre-existing uncommitted governance files (`DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`) are preserved at commit `925a095` (`2026-10-17`) and lag behind `HEAD` (`d2d048a507370a787a750984e3c1a89690078613`, `2026-10-19`). Always reconcile against live `git rev-parse HEAD` and [`PROJECT_SNAPSHOT.md`](PROJECT_SNAPSHOT.md).

---

## 1. Quick Phase B Context Pack Selector (By Task Type)

Use this table at the end of **Phase A** to select **only** the Phase B documents applicable to your authorized task:

| Task Category | Phase B Documents to Read | Documents to SKIP |
| :--- | :--- | :--- |
| **Owner UAT Preparation (`OWNER-UAT-PREP-001`)** | `NEXT_TASK.md`, `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`, `docs/testing/FINANCIAL_GOLDEN_CASES.md`, `artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md` (§4, §8–§10), `FINAL_ACCEPTANCE_GATE_REPORT.md`, `artifacts/local/independent-qa-final-retest/OWNER_UAT_GUIDE.md` | All `backend/` & `frontend/` source code, `SUMI_GAP_REGISTER.md`, `DEV_BATCHES.md`, `development-train-01..04`, `V3_FINAL_HANDOFF_REPORT_2026-09-12.md` |
| **Session Context / Governance Maintenance (`CTX-*`)** | `docs/operations/agent-sessions/*.md`, `artifacts/local/session-ops/<prior-ctx-task>/SESSION_HANDOFF.md`, `FINAL_BASELINE_HANDOFF.md` (§1, §6–§7) | Application source code, unrelated historical train folders |
| **Replay / Chart / Indicator / Drawing UI** | `ADR-001`, `ADR-FE-001`, `ADR-WS-001`, `ADR-CHART-001`, `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` (`G-*`, `R-*`, `I-*`, `D-*`), targeted `frontend/src/` modules | Market data provider audit, historical `P1–P10` research reports |
| **Execution / Settlement / Backtest / Financial Math** | `ADR-EXEC-001`, `ADR-SETTLE-001`, `ADR-TX-001` (+ Addenda), `docs/testing/FINANCIAL_GOLDEN_CASES.md` (`G-01..G-12`), `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` (`T-01..T-05`), targeted `backend/app/` modules | Unrelated UI plugin files, historical train logs |
| **Market Data Provider / Import / Quarantine** | `ADR-002`, `ADR-PROV-001` (+ Addendum 01), `ADR-TX-001` (+ Addenda), `docs/research/DORAEMON_MARKET_DATA_AUDIT.md`, `docs/research/data_capability_matrix.csv` | Frontend chart primitives, historical UI UAT bundles |

---

## 2. Repository Governance & Session Operations

| File Path | Disk Status | Authority Level | Loading Phase | Purpose & Reconciliation Notes |
| :--- | :---: | :--- | :---: | :--- |
| [`AGENTS.md`](../../../AGENTS.md) | `EXISTS` | `RATIFIED DECISION` / `VERIFIED CURRENT STATE` | **Phase A (Mandatory)** | Top-level operating charter: product standard, non-negotiable invariants, architecture boundaries, escalation rules, and safety rules. |
| [`docs/operations/agent-sessions/SESSION_PROTOCOL.md`](SESSION_PROTOCOL.md) | `EXISTS` | `VERIFIED CURRENT STATE` | **Phase A (Mandatory)** | Hardened two-phase context loading protocol, 6-rank authority hierarchy, 12-attribute task boundary contract, and workspace-local `test_db/` rules. |
| [`docs/operations/agent-sessions/PROJECT_SNAPSHOT.md`](PROJECT_SNAPSHOT.md) | `EXISTS` | `VERIFIED CURRENT STATE` | **Phase A (Mandatory)** | Verified current baseline (`HEAD` `d2d048a`), dirty working-tree reconciliation, `backend/sumi.db` checksum, and separation of current vs. historical evidence. |
| [`docs/operations/agent-sessions/TASK_REGISTRY.md`](TASK_REGISTRY.md) | `EXISTS` | `VERIFIED CURRENT STATE` | **Phase A (Mandatory)** | Canonical task ledger with explicit status semantics (`PROPOSED`, `AUTHORIZED`, `IN_PROGRESS`, `BLOCKED`, `COMPLETED`, `AWAITING_OWNER_REVIEW`, `DEFERRED`) and full 12-attribute task cards. |
| [`docs/operations/agent-sessions/CONTEXT_INDEX.md`](CONTEXT_INDEX.md) | `EXISTS` | `VERIFIED CURRENT STATE` | **Phase A Step 6 (Lookup Only)** | Directory mapping tasks to Phase B documents and authority classifications. |
| [`docs/operations/agent-sessions/NEXT_TASK.md`](NEXT_TASK.md) | `EXISTS` | `PROPOSED / UNRATIFIED` | **Phase B (When Relevant)** | Full 12-attribute specification of the next candidate task (`OWNER-UAT-PREP-001`). **Not** an authorization to execute. |
| [`docs/operations/agent-sessions/HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md) | `EXISTS` | `VERIFIED CURRENT STATE` | **Step 6 (Handoff Writing)** | Required schema for `SESSION_HANDOFF.md` and `SESSION_RESULT.json` under `artifacts/local/session-ops/<task-id>/`. |
| [`PLANS.md`](../../../PLANS.md) | `EXISTS` | `RATIFIED DECISION` | **Phase B (Dev Batches Only)** | Execution-plan (`ExecPlan`) format specification for code implementation batches. |
| [`docs/INDEX.md`](../../INDEX.md) | `EXISTS` | `HISTORICAL EVIDENCE` | **Skip (Superseded)** | Legacy table of contents containing stale `file:///e:/Workspace/sumi/` Windows links. Use `CONTEXT_INDEX.md` instead. |

---

## 3. Architecture & Ratified ADRs (`Phase B — Read Only Applicable ADRs`)

| File Path | Disk Status | Authority Level | Governing Scope |
| :--- | :---: | :--- | :--- |
| [`docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`](../../ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-001**: Option C Replay/Chart UI rebuild keeping Lightweight Charts v5, backend `IndicatorEngine`, and custom primitive drawing boundary. |
| [`docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`](../../ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-002**: Vietnam market data provider evaluation and `MarketDataProviderAdapter` boundary contract preserving offline CafeF autonomy. |
| [`docs/architecture/decisions/ADR-TX-001-transaction-boundary.md`](../../architecture/decisions/ADR-TX-001-transaction-boundary.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-TX-001**: `atomic()` transaction boundary, `PRAGMA foreign_keys=ON`, absolute `<BACKEND_DIR>/sumi.db` path guard, 20-model Alembic registration, and read-only import preview. |
| [`docs/architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md`](../../architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-TX-001 Addendum 01**: Structured `UPDATE:{prior_source}\|{prior_timestamp_iso}` mutation provenance and fail-closed rollback restoration (`DEC-01`). |
| [`docs/architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md`](../../architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-TX-001 Addendum 02**: Symmetrical cross-workflow rollback conflict checks between `ImportRun` and `SyncRun` (`DEC-01`). |
| [`docs/architecture/decisions/ADR-PROV-001-synthetic-policy.md`](../../architecture/decisions/ADR-PROV-001-synthetic-policy.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-PROV-001**: Default-deny policy for synthetic data providers (`SSI`, `Vnstock` stubs) unless `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`, with `quarantine:%` dataset isolation. |
| [`docs/architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md`](../../architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-PROV-001 Addendum 01**: Option A in-place quarantine promotion when canonical data arrives and fail-closed rejection on multi-candidate same-date collisions (`DEC-01`). |
| [`docs/architecture/decisions/ADR-EXEC-001-execution-semantics.md`](../../architecture/decisions/ADR-EXEC-001-execution-semantics.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-EXEC-001**: Canonical execution model (`BacktestExecutionKernel`: `Signal[T] -> Order QUEUED MARKET_NEXT_OPEN -> Fill Open[T+1]`, cutoff `N-1 -> UNFILLED_AT_CUTOFF`, `fill_estimate` sizing). |
| [`docs/architecture/decisions/ADR-SETTLE-001-settlement-market-rules.md`](../../architecture/decisions/ADR-SETTLE-001-settlement-market-rules.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-SETTLE-001**: Single `MarketProfile` (`fee_buy=0.15%`, `fee_sell=0.15%`, `tax_sell=0.10%`, `lot=100`, `slippage=0.0`) and `SettlementEngine` (`T+1.5` calendar settlement displayed as `T+1 · về sau 1 phiên`). |
| [`docs/architecture/decisions/ADR-FE-001-state-ownership.md`](../../architecture/decisions/ADR-FE-001-state-ownership.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-FE-001**: Frontend state ownership via URL `?session=` authority, composite TanStack Query keys (`replayQueryKeys`), navigation abort tokens, and session-switch cleanup. |
| [`docs/architecture/decisions/ADR-WS-001-ws-reliability.md`](../../architecture/decisions/ADR-WS-001-ws-reliability.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-WS-001**: Backend WebSocket replay state machine (`seq` envelope, speed clamping, `ping`/`pong`, `sync`) and frontend exponential-backoff reconnect with command queueing. |
| [`docs/architecture/decisions/ADR-CHART-001-chart-kernel.md`](../../architecture/decisions/ADR-CHART-001-chart-kernel.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-CHART-001**: `ChartKernel` thin facade encapsulating `createChart`, `MappingResult` indicator rendering contract, and `ReplayWorkspace` decomposition. |
| [`docs/architecture/decisions/ADR-API-001-contract-governance.md`](../../architecture/decisions/ADR-API-001-contract-governance.md) | `EXISTS` | `RATIFIED DECISION` | **ADR-API-001**: Standardized `{code, message, field_errors, detail}` error envelope, explicit Pydantic `response_model` on GET routes, OpenAPI contract snapshots, and 422 normalization. |
| [`docs/architecture/TARGET_ARCHITECTURE_V3.md`](../../architecture/TARGET_ARCHITECTURE_V3.md) | `EXISTS` | `RATIFIED DECISION` | Target V3 domain, financial, and frontend architecture specification (read only for broad architecture lookups). |
| [`docs/architecture/SUMI_GAP_REGISTER.md`](../../architecture/SUMI_GAP_REGISTER.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Historical register of `GAP-*` items closed across `B00–B51` (**Skip unless tracing historical gap IDs**). |

---

## 4. Product Requirements, Financial Golden Cases & Data Audits (`Phase B — Selective`)

| File Path | Disk Status | Authority Level | Purpose & Scope |
| :--- | :---: | :--- | :--- |
| [`docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`](../../PRODUCT_ACCEPTANCE_CRITERIA_V3.md) | `EXISTS` | `RATIFIED DECISION` | Mandatory release gates: Global (`G-01..G-05`), Replay (`R-01..R-05`), Indicator Manager (`I-01..I-13`), Drawing System (`D-01..D-11`), Trading Practice (`T-01..T-05`), and viewport rules (`1440×1000`, `1280×800`, `<1280px` warning). |
| [`docs/testing/FINANCIAL_GOLDEN_CASES.md`](../../testing/FINANCIAL_GOLDEN_CASES.md) | `EXISTS` | `RATIFIED DECISION` | Hand-calculated Financial Golden Cases `G-01..G-12`: Long win (`G-01`), Long loss & breakeven `net_pnl > 0` rule (`G-02`), Symmetric Short (`G-03`), Next-open SL (`G-04`), Cutoff `N-1` (`G-05`), Weekend/Holiday `T+1.5` settlement (`G-06`, `G-07`), Zero-loss `PF = N/A` (`G-08`), `N < 30` sample gate (`G-09`), Annualized Sharpe `sqrt(252)` (`G-10`), `fill_estimate` lot-100 sizing (`G-11`), and `Sweep == Batch == Single` parity (`G-12`). |
| [`docs/testing/WEBSOCKET_TEST_MATRIX.md`](../../testing/WEBSOCKET_TEST_MATRIX.md) | `EXISTS` | `RATIFIED DECISION` | WebSocket reliability and chaos test matrix (`WS-01..WS-10`). |
| [`docs/research/DORAEMON_MARKET_DATA_AUDIT.md`](../../research/DORAEMON_MARKET_DATA_AUDIT.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Read **only** before tasks touching external Doraemon/provider data capabilities. |
| [`docs/research/data_capability_matrix.csv`](../../research/data_capability_matrix.csv) | `EXISTS` | `HISTORICAL EVIDENCE` | Provider endpoint, schema, and coverage matrix (read **only** for provider tasks). |

---

## 5. Product Completion Roadmap & Lagging Uncommitted Governance Records (`Phase B — Reconcile With Care`)

> [!WARNING]
> **Protected Uncommitted Working-Tree Files (`3` Files)**:
> The three files marked `PROTECTED UNCOMMITTED` below contain working-tree edits from `2026-10-17` (commit `925a095`) and **lag behind `HEAD` (`d2d048a507370a787a750984e3c1a89690078613`, `2026-10-19`)**. Do **not** edit, stage, reset, or commit these three files unless explicitly authorized by the Product Owner. Always use [`PROJECT_SNAPSHOT.md`](PROJECT_SNAPSHOT.md) and [`FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md) for post-`925a095` state (`d2d048a`).

| File Path | Disk Status | Authority Level | Purpose & Lag / Protection Status |
| :--- | :---: | :--- | :--- |
| [`docs/plans/product-completion/DECISION_LOG.md`](../../plans/product-completion/DECISION_LOG.md) | `EXISTS` | `RATIFIED DECISION` *(Lagging Working-Tree File — Protected)* | Authoritative log of ratified decisions `DEC-01..DEC-10`, `DEC-TRAIN01..04`, and `DEC-REM-01..08`. Uncommitted diff (`+15 -0`) ends at `DEC-10` (`925a095`, `2026-10-17`); does not yet record `d2d048a`. |
| [`docs/plans/product-completion/EXECUTION_STATE.md`](../../plans/product-completion/EXECUTION_STATE.md) | `EXISTS` | `HISTORICAL EVIDENCE` *(Lagging Working-Tree File — Protected)* | Detailed execution log through `925a095` (`854` BE / `272` FE tests). Uncommitted diff (`+31 -4`) lags behind `HEAD` `d2d048a` (`950` BE / `338` FE tests). |
| [`docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`](../../plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md) | `EXISTS` | `HISTORICAL EVIDENCE` *(Lagging Working-Tree File — Protected)* | 5-dimensional status board for `B00–B51`. Uncommitted diff (`+10 -5`) stops at `925a095` (`2026-10-17`). |
| [`docs/plans/product-completion/SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`](../../plans/product-completion/SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md) | `EXISTS` | `RATIFIED DECISION` (Journeys/Boundaries) / `PROPOSED / UNRATIFIED` (Draft sections) | Master plan defining the 5 core user journeys and `W0–W5` completion architecture. |
| [`docs/plans/DEV_BATCHES.md`](../../plans/DEV_BATCHES.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Historical batch specifications `B00–B51` (**Skip unless auditing batch history**). |

---

## 6. Session-Ops Handoffs & Historical QA Evidence (`Phase B — Selective Lookup`)

| Path | Disk Status | Authority Level | Scope & Contents |
| :--- | :---: | :--- | :--- |
| [`artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md) | `EXISTS` | `VERIFIED CURRENT STATE` (Active Session Handoff) | Handoff for `CTX-HARDEN-002` documenting progressive context loading, task boundary hardening, status semantics reconciliation, workspace-local `test_db/` standard, and `OWNER-UAT-PREP-001` bootstrap dry-run. |
| [`artifacts/local/session-ops/CTX-HARDEN-002/BOOTSTRAP_DRY_RUN_REPORT.md`](../../../artifacts/local/session-ops/CTX-HARDEN-002/BOOTSTRAP_DRY_RUN_REPORT.md) | `EXISTS` | `VERIFIED CURRENT STATE` | Step-by-step zero-memory bootstrap simulation for a fresh session receiving `OWNER-UAT-PREP-001`. |
| [`artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Initial session-context bootstrap handoff (`2026-10-19T12:48:00+07:00`). Superseded in protocol rules by `CTX-HARDEN-002`. |
| [`artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md) | `EXISTS` | `HISTORICAL EVIDENCE` (Primary Code Baseline Handoff for `HEAD` `d2d048a`) | Documents commit `d2d048a` (`46` files), `SQA-001..005, 009..018` fixes, `950` BE / `338` FE tests, `19/19` browser checks, `4` deferred `LOW` issues (`SQA-006..008, 019`), and 5 recommended Owner UAT journeys. *(Note: Replace its `/tmp/sumi-owner-uat/` example with `artifacts/local/session-ops/<task-id>/test_db/` per `SESSION_PROTOCOL.md` §5.)* |
| [`artifacts/local/full-browser-qa-acceptance-gate/FINAL_ACCEPTANCE_GATE_REPORT.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_ACCEPTANCE_GATE_REPORT.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Detailed Independent QA Lead report for `d2d048a` (`SQA-001`, `SQA-011`, `SQA-017`, `G-01..G-12`, `logs/`, `screenshots/`). |
| [`artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db`](../../../artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db) | `EXISTS` | `HISTORICAL EVIDENCE` (Seeded Source Fixture DB) | Pre-seeded SQLite database containing `FPT`, `SSI`, `VCI`, `VNINDEX`, `ZERO_PNL_SYM` for copying into `artifacts/local/session-ops/<task-id>/test_db/` during UAT preparation. |
| [`artifacts/local/independent-qa-final-retest/OWNER_UAT_GUIDE.md`](../../../artifacts/local/independent-qa-final-retest/OWNER_UAT_GUIDE.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Supplemental Owner UAT guide from earlier commit `925a095` (`DEF-001..012`). |
| [`artifacts/local/independent-qa-final-retest/FINAL_INDEPENDENT_RETEST_REPORT.md`](../../../artifacts/local/independent-qa-final-retest/FINAL_INDEPENDENT_RETEST_REPORT.md) | `EXISTS` | `HISTORICAL EVIDENCE` | Independent QA retest report for commit `925a095` (**Skip unless auditing `DEF-*` history**). |
| [`artifacts/local/development-train-05/`](../../../artifacts/local/development-train-05/) | `EXISTS` | `HISTORICAL EVIDENCE` | Wave `W5` UAT bundle for commit `bf0bbc6` (**Skip unless auditing `W5` history**). |
| [`artifacts/local/development-train-01/`](../../../artifacts/local/development-train-01/) .. [`development-train-04/`](../../../artifacts/local/development-train-04/) | `EXISTS` | `HISTORICAL EVIDENCE` | Historical Wave `W0-A` through `W4` logs (**Skip during normal sessions**). |
| [`docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md`](../../V3_FINAL_HANDOFF_REPORT_2026-09-12.md) & [`docs/dev-program/STATE.json`](../../dev-program/STATE.json) | `EXISTS` | `HISTORICAL EVIDENCE` | Pre-October 2026 `P1–P10` research program handoff (`HISTORICAL_ACCEPTED_SUPERSEDED` — **Skip during normal sessions**). |
