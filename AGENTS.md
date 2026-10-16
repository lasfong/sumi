# Sumi repository operating rules

## Product standard

Sumi is a local-first manual replay and backtesting product for serious technical-analysis practice on Vietnam market data. Do not call the product "TradingView-like" unless every relevant requirement in `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` passes in browser UAT.

## Canonical sources

Read these before planning or changing product behavior:

1. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
2. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
3. `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`
4. `docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md`
5. `PLANS.md` for execution-plan format

## Non-negotiable invariants

- Never leak future candles. Replay APIs must return data only through `current_index`; do not send all candles and slice in the browser.
- Keep indicator calculation authoritative in backend `IndicatorEngine` unless an approved architecture decision changes this.
- Do not mutate `backend/sumi.db` in automated tests or product UAT. Use a temporary database.
- Do not add a chart/drawing dependency without a recorded spike result, license review, and provider-boundary design.
- Do not declare a feature complete from unit tests alone. User-facing chart work requires browser evidence.
- Preserve local-first behavior: no telemetry and no user market/trading data sent to external services.

## Development workflow

- Autonomous program entry point: `docs/dev-program/START.md`. When the owner launches it, the orchestrator may dispatch bounded batches, obtain independent internal review, repair and advance without owner handoff after each batch. Existing safety/data/acceptance gates still apply. Legacy `docs/dev-prompts/` STOP instructions are historical, not the next-work queue. Internal review does not authorize owner-only decisions. Only one production writer at a time; persist progress in `docs/dev-program/STATE.json`.

- Work in one bounded batch at a time, with an ExecPlan following `PLANS.md`.
- A batch must deliver a complete vertical capability, not scattered partial changes.
- Keep implementation work in a dedicated DEV task. By default it uses the current checkout and branch; create a branch/worktree only when the user explicitly requests isolation or parallel writes. The reviewer/orchestrator task should not concurrently edit the same files.
- Before coding, record scope, affected modules, acceptance IDs, rollback strategy, and exact verification commands.
- Before any batch that depends on Doraemon data or a provider capability, read `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` and `docs/research/data_capability_matrix.csv`, then run a read-only targeted freshness check for only the exact source, endpoint, fields, symbols/universe, and date range the batch will use. Do not repeat the full Doraemon audit.
- If the targeted check differs from the recorded schema, freshness, coverage, semantics, access, or rights classification, update the audit evidence and capability matrix before coding. Stop for reviewer direction when a mandatory capability has regressed or remains unverified.
- After coding, review the diff against the ExecPlan and acceptance IDs; document deviations.
- Never hide a known failure by weakening a test, removing an assertion, or changing acceptance criteria in the same implementation batch without reviewer approval.

## Escalation rules

DEV must stop and return to reviewer when:

- A provider fails a mandatory spike criterion.
- Persistence migration could lose existing sessions/drawings.
- A new dependency changes license/security posture.
- Fixing the batch requires changing a backend contract outside scope.
- Product acceptance criteria are internally inconsistent or infeasible.

## Required verification

Fast technical gate:

```powershell
.\scripts\verify-v2.ps1
```

Comprehensive browser E2E UAT:

```powershell
.\scripts\run-comprehensive-uat.ps1
```

For Replay/Chart/Indicator/Drawing changes, the product UAT result must be green and screenshots must be reviewed at 1440×1000. Add focused UAT assertions for new behavior rather than relying on "page is not blank."

## Architecture boundaries

- Backend business logic stays out of FastAPI routes.
- `ReplayPage` is an application composition surface, not a place for chart engine, persistence, or drawing geometry logic.
- Chart-library calls belong behind chart/provider adapters.
- Indicator product state must be explicit and serializable: identity, parameters, pane, visibility, style, order.
- Drawing product state must be versioned and independent of a specific community provider's raw JSON.
- Keep UI labels and pane semantics separate from backend dataframe column names.

## Definition of done

A product batch is done only when:

1. The specified acceptance IDs pass.
2. Unit/integration tests, lint, and build pass.
3. Browser UAT passes with no page/console errors.
4. Required screenshots and machine-readable results are retained.
5. No unrelated working-tree changes are included.
6. The ExecPlan progress, decisions, and verification evidence are updated.
7. The reviewer/orchestrator has inspected the diff and evidence.

## Sumi V3 Project Execution Charter & Governance

All Antigravity sessions and Development Trains must recover and follow the established Sumi V3 product completion strategy from these canonical governance records:

### Canonical V3 Governance & Architecture References

- **Master Plan**: [`docs/plans/product-completion/SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`](docs/plans/product-completion/SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md)
- **Batch Dependency & Status Board**: [`docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`](docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md)
- **Live Execution State**: [`docs/plans/product-completion/EXECUTION_STATE.md`](docs/plans/product-completion/EXECUTION_STATE.md)
- **Decision Log**: [`docs/plans/product-completion/DECISION_LOG.md`](docs/plans/product-completion/DECISION_LOG.md)
- **Financial Golden Cases (`G-01..G-12`)**: [`docs/testing/FINANCIAL_GOLDEN_CASES.md`](docs/testing/FINANCIAL_GOLDEN_CASES.md)
- **Ratified Architecture Decision Records (`docs/architecture/decisions/`)**:
  - [`ADR-TX-001`](docs/architecture/decisions/ADR-TX-001-transaction-boundary.md) (+ [`ADDENDUM-01`](docs/architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md), [`ADDENDUM-02`](docs/architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md))
  - [`ADR-PROV-001`](docs/architecture/decisions/ADR-PROV-001-synthetic-policy.md) (+ [`ADDENDUM-01`](docs/architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md))
  - [`ADR-EXEC-001`](docs/architecture/decisions/ADR-EXEC-001-execution-semantics.md)
  - [`ADR-SETTLE-001`](docs/architecture/decisions/ADR-SETTLE-001-settlement-market-rules.md)
  - [`ADR-FE-001`](docs/architecture/decisions/ADR-FE-001-state-ownership.md)
  - [`ADR-WS-001`](docs/architecture/decisions/ADR-WS-001-ws-reliability.md)
  - [`ADR-CHART-001`](docs/architecture/decisions/ADR-CHART-001-chart-kernel.md)
  - [`ADR-API-001`](docs/architecture/decisions/ADR-API-001-contract-governance.md)

### Mandatory Development Train Protocol

Every new Development Train and session must:

1. **Recover Context First**: Read the product goals, roadmap, batch dependencies, and current execution state (`SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`, `EXECUTION_STATE.md`, `DECISION_LOG.md`) before planning or coding.
2. **Maintain Full Traceability**: Trace every task explicitly across `Product Requirement → Gap ID → Batch ID → Acceptance Criteria / Golden Case`.
3. **Honor Ratified Decisions**: Respect all previously approved architectural and financial decisions in `DECISION_LOG.md` and `docs/architecture/decisions/`.
4. **Enforce Orthogonal Status Discipline**: Strictly distinguish between `Implementation Complete`, `Test Verified`, `Git Committed`, and `Product Accepted` (including browser E2E UAT). Never conflate unit/integration pass with full product acceptance.
5. **Single Source of Execution Truth**: Update the central execution records (`EXECUTION_STATE.md`, `DECISION_LOG.md`, and `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`) in place without creating parallel or duplicate roadmaps.
6. **Bounded Escalation**: Resolve routine engineering bugs and test regressions autonomously; escalate only genuine Product Owner or Technical Architect policy decisions.
7. **Self-Contained Subagent Prompts**: Never assume a newly spawned subagent remembers any prior Antigravity session; always provide explicit file paths, ratified contracts, invariants, and verification commands.
8. **Strict Git & Workspace Safety**: Never perform destructive git/file cleanup (`git reset --hard`, `git checkout --`, `git clean`) or create Git commits without explicit Product Owner authorization.
