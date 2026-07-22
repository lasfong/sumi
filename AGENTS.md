# Sumi repository operating rules

## Product standard

Sumi is a local-first manual replay and backtesting product for serious technical-analysis practice on Vietnam market data. Do not call the product “TradingView-like” unless every relevant requirement in `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` passes in browser UAT.

## Canonical sources

Read these before planning or changing product behavior:

1. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
2. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
3. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
4. `docs/DEVELOPMENT_OPERATING_MODEL.md`
5. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
6. `PLANS.md` for execution-plan format

Older V2 completion/release documents are historical evidence. They do not override V3 acceptance criteria.

## Non-negotiable invariants

- Never leak future candles. Replay APIs must return data only through `current_index`; do not send all candles and slice in the browser.
- Keep indicator calculation authoritative in backend `IndicatorEngine` unless an approved architecture decision changes this.
- Do not move, retag, or rewrite `v2.0.0-rc2`.
- Do not mutate `backend/sumi.db` in automated tests or product UAT. Use a temporary database.
- Do not add a chart/drawing dependency without a recorded spike result, license review, and provider-boundary design.
- Do not declare a feature complete from unit tests alone. User-facing chart work requires browser evidence.
- Preserve local-first behavior: no telemetry and no user market/trading data sent to external services.

## Development workflow

- Work in one bounded batch at a time, with an ExecPlan following `PLANS.md`.
- A batch must deliver a complete vertical capability, not scattered partial changes.
- Keep implementation work in a dedicated DEV task. By default it uses the current checkout and branch; create a branch/worktree only when the user explicitly requests isolation or parallel writes. The reviewer/orchestrator task should not concurrently edit the same files.
- Before coding, record scope, affected modules, acceptance IDs, rollback strategy, and exact verification commands.
- After coding, review the diff against the ExecPlan and acceptance IDs; document deviations.
- Never hide a known failure by weakening a test, removing an assertion, or changing acceptance criteria in the same implementation batch without reviewer approval.

## Required verification

Fast technical gate:

```bash
./scripts/verify-v2.sh
```

Deterministic product UAT (starts isolated backend/frontend and retains artifacts):

```bash
./scripts/run-product-uat.sh
```

Full product gate:

```bash
./scripts/verify-product.sh
```

For Replay/Chart/Indicator/Drawing changes, the product UAT result must be green and screenshots must be reviewed at 1440×1000. Add focused UAT assertions for new behavior rather than relying on “page is not blank.”

## Architecture boundaries

- Backend business logic stays out of FastAPI routes.
- `ReplayPage` is an application composition surface, not a place for chart engine, persistence, or drawing geometry logic.
- Chart-library calls belong behind chart/provider adapters.
- Indicator product state must be explicit and serializable: identity, parameters, pane, visibility, style, order.
- Drawing product state must be versioned and independent of a specific community provider’s raw JSON.
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
