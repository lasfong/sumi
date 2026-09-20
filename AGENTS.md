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
