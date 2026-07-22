# Batch 4 DEV prompt — Integrated Trading-Practice Workflow

You are the implementation agent for one bounded Sumi V3 batch in `/Users/mizuhara/workspace/sumi`. This file is the complete authority for a new DEV session with no conversation history.

## Authorization and stop boundary

Implement **Batch 4 only: Integrated Trading-Practice Workflow**. Batch 1 Replay foundation, Batch 2 Indicator Manager, and Batch 3 Drawing MVP are approved baselines and blocking regressions. Batch 5, release work, broad backtesting-engine work, and any claim that Sumi is product-complete, professionally ready, release-ready, or “TradingView-like” are unauthorized.

Use the current checkout and branch. Do not create or switch a branch/worktree. Do not stage, commit, push, merge, reset, clean, checkout files, move/retag `v2.0.0-rc2`, or discard unrelated dirty-tree changes. End at the Reviewer gate.

Sumi is a local-first manual replay and backtesting workstation for serious personal technical-analysis practice on Vietnam market data. The goal is a coherent, dependable practice workflow—not feature-count parity with another product.

## Mandatory reading order

Read these files completely before planning or editing product code:

1. `AGENTS.md`
2. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
5. `docs/DEVELOPMENT_OPERATING_MODEL.md`
6. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
7. `PLANS.md`
8. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
9. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`
10. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
11. `docs/exec-plans/BATCH_2_PROFESSIONAL_INDICATOR_MANAGER.md`
12. `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`
13. `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`

The V3 acceptance criteria override historical V2 completion language. Preserve every accepted Batch 1–3 assertion ID, behavior, architecture boundary, and artifact expectation.

## First action: provenance and workspace audit

Before design, record branch, HEAD, dereferenced `v2.0.0-rc2^{}`, dirty-tree inventory, `backend/sumi.db` SHA-256, package/dependency state, and the latest accepted product-UAT result. Do not assume dirty files belong to this batch.

Audit the current browser workspace at 1440×1000 and 1280×800 and inspect at least:

- Replay header, visible symbol/date/bar/OHLCV, navigation/autoplay, chart geometry and vertical scrolling;
- indicator manager and pane chrome;
- drawing rail, selection inspector, keyboard ownership and provider snapshot;
- `TradeControls`, decision modal, Pending Orders, Open Position/P&L, Decision Log and the separate journal API/page;
- loading, empty, disabled, validation, success, rejection and transport-error states;
- browser reload/resume and forward/rewind behavior;
- current API/domain contracts in `frontend/src/api/decisionApi.ts`, `journalApi.ts`, replay types/components/controller, `backend/app/api/decisions.py`, `journal.py`, `ReplayService`, `TradeLifecycleService`, schemas/models, and lifecycle tests.

Repository reality already observed by the Reviewer must guide the ExecPlan:

- the backend supports BUY/SELL/HOLD/SKIP plus extended actions, market-at-close and LIMIT orders, open positions/P&L, T+2 sell rejection, pending-order matching, decisions, trades, and journal CRUD;
- Replay currently refetches candles/position/orders after navigation and renders trade decision markers, but the integrated UI does not yet expose the standalone journal contract as a coherent in-workspace journal/checklist;
- `ReplayService.previous_candle()` currently moves `current_index` backward without reversing or projecting decisions, executions, orders, positions, trades, cash, or journal state. This is a demonstrable contract gap for R-04/T-04, not permission for a broad rewrite.

## Required ExecPlan before product code

Create `docs/exec-plans/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW.md` following `PLANS.md` before any product-code edit. Record:

- scope/out-of-scope, affected modules, acceptance IDs, current gaps, user workflow and information hierarchy;
- state ownership and synchronization for candles, indicators, drawings, markers, decisions, orders, positions, trades, journal/checklist, cash and replay index;
- precise forward, rewind, reload and resume semantics, including how future-created state is hidden, projected, reversed, or otherwise reconciled without data loss or future leak;
- the narrow backend-service change, if required, and why the existing contract is defective; keep business logic out of FastAPI routes;
- accepted chart geometry at 1440×1000 and 1280×800, compact overflow/scroll policy, and explicit mobile limitation;
- keyboard/focus ownership, error/loading/disabled rules, test/UAT mapping, rollback, stop conditions and exact commands.

Do not begin product code until the ExecPlan is sufficiently concrete to review.

## Required outcome and acceptance mapping

Deliver one complete vertical workflow satisfying T-01–T-05, R-02, R-04, and G-01–G-05 while preserving all accepted Replay/Indicator/Drawing behavior.

### T-01 — chart remains primary

- Establish one coherent hierarchy for header/context, chart, drawing/indicator controls, trading state, journal/checklist and secondary detail.
- The chart remains the dominant practice surface and retains the approved pane minimums/scroll behavior. Trade/journal panels must not cover replay controls or make chart interaction unusable.
- At 1440×1000, core context and actions must be visible without accidental clipping. At 1280×800, use an explicit responsive rail/tab/drawer/scroll policy that keeps chart, replay, trade and journal access understandable.
- State an honest mobile limitation; do not claim a phone-optimized professional chart workstation unless it is actually implemented and verified.

### T-02 — understandable actions and lifecycle feedback

- Buy, Sell, Hold and Skip must be obvious in current symbol/date/price context. CLOSE and supported advanced actions may remain available only when their semantics and eligibility are clear.
- Trade inputs must validate quantity, order type, price, stop/target and required context before submission. Prevent duplicate submission and show honest pending/success/rejection/transport states.
- Pending LIMIT orders must show side, quantity, requested price, status and what advances can fill/reject them. Executed/cancelled/rejected states must not masquerade as pending.
- Open position state must show symbol, quantity, average price, current price, realized/unrealized P&L, and actionable T+2 availability/feedback. A pre-T+2 sell must visibly explain the constraint and leave state consistent; an eligible sell must update all related state.
- Hold/Skip must record an intentional decision without creating an order/position marker that implies execution.

### T-03 — journal and checklist without context loss

- Provide a Sumi-owned in-workspace journal/checklist surface reachable without route loss, session loss, replay reset or chart destruction.
- Reuse the existing decision and journal contracts where sound. Define checklist fields explicitly and serialize them through an existing appropriate contract or a narrowly justified service/schema change; do not hide canonical checklist state only in DOM test outputs.
- Users must be able to record, view and reload a note/checklist tied to the current session and, when applicable, a decision/trade. Show empty/loading/error/saved states and retain the chart context while the surface opens/closes.
- Avoid duplicating competing journal authorities between `Decision.note/reason` and `/journal`; document ownership and any migration/compatibility choice.

### T-04 and R-04 — synchronized navigation and restoration

- Forward and rewind must synchronize visible candles, active indicators, drawings, trade markers, decisions, pending orders, positions/P&L, cash/T+2 feedback, and journal/checklist without duplicate requests, duplicate markers, stale panels, races or future state leakage.
- Markers must reflect executed trade/decision semantics accurately and never show a future decision on an earlier replay bar.
- Reload/resume must restore the complete integrated workspace for the session: replay index, chart context, indicator document, drawing document, orders, position/trade state, decision markers, and journal/checklist.
- Test single-step, ±5, autoplay/pause, pending-order fill, BUY then pre-T+2 rejection then eligible SELL, rewind across a decision/execution boundary, forward again, reload, and resume.
- Because current rewind only decrements the index, the batch may make a **scoped service-layer correction** after the ExecPlan defines deterministic semantics and tests. Do not put lifecycle logic in routes, do not fabricate client-only state, and do not expand into a generic event-sourcing platform or backtesting rewrite. If correctness requires a database migration, destructive record rewrite, or unresolved accounting architecture decision, stop and request Reviewer direction.

### R-02 — obvious current context

- Current symbol, replay date, 1-based bar index/total visible bars, O/H/L/C and Volume must be readable without a crosshair at both required viewports.
- Trade and journal surfaces must visibly inherit the same context so a user cannot unknowingly act on another symbol/date/bar.

### T-05 — complete practice-session path

Prove a realistic manual sequence using real UI actions: resume/create session; advance to warmup; configure/retain indicators and drawings; record a checklist/journal observation; BUY or place/fill a LIMIT order; observe marker/order/position/P&L; receive T+2 rejection; advance until eligible; SELL/CLOSE; record review notes; rewind/forward; reload/resume; continue without a missing core action or runtime error. Scope the browser run to deterministic evidence rather than waiting 30 wall-clock minutes, but cover the actions needed for an actual 30-minute practice session.

### G-01–G-05 and retained invariants

- Full backend/frontend tests, lint, build, deterministic UAT and all product gates pass.
- UAT and automated backend work use a temporary database; never mutate `backend/sumi.db`.
- Every successful or failed UAT retains `results.json`, screenshots and runtime logs.
- No P0/P1 remains in the Batch 4 workflow. No accepted Batch 1–3 assertion is removed, renamed, skipped, weakened, or made nonblocking.
- Remain local-first: no telemetry and no user market/trading/journal/checklist data leaves the local product.
- Never leak future candles. Indicator calculation remains backend-authoritative. Drawing documents remain provider-independent and use only visible replay candles. Keep chart/provider calls behind adapters and business logic out of routes.

## Architecture and dependency boundaries

- `ReplayPage` remains a route composition surface. Keep trade/replay orchestration out of a new JSX monolith; separate domain/service/controller and presentational ownership where it materially clarifies synchronization.
- Prefer the existing API, TanStack Query, store, chart facade, Sumi indicator/drawing domains and backend lifecycle services. Do not add a UI/chart/state dependency without an approved decision and license/provider-boundary review.
- No chart-library switch, private Lightweight Charts API, telemetry, cloud persistence, broad backend rewrite or unrelated cleanup.
- A narrow backend contract/service fix is authorized only when a focused failing test demonstrates the defect and the ExecPlan records scope, rollback and compatibility. Escalate instead of silently changing trading/accounting semantics.

## Required tests and browser evidence

Add focused frontend tests for the integrated view model, responsive composition, keyboard/focus isolation, submission deduplication, validation, marker filtering, query synchronization, journal/checklist persistence, reload/resume and all error/disabled states. Add backend service/integration tests for any lifecycle contract change, including forward/rewind and T+2/order/position/cash consistency. Do not weaken existing tests.

Extend `scripts/product-uat.mjs` additively. Retain every existing assertion ID exactly once and keep all accepted IDs blocking. New checks must use real UI actions and exact API/machine state; hidden declarations are diagnostic only, not user-facing proof.

Required browser scenarios include:

- real BUY, SELL, HOLD, SKIP and LIMIT-order paths;
- pending fill/rejection and exact marker/order/position/P&L state;
- pre-T+2 rejection and later eligible sale;
- journal/checklist create, association, close/reopen and reload;
- forward, rewind across lifecycle boundaries, forward again, autoplay/pause, ±5 and reload/resume;
- no duplicate markers/orders/requests, no future decision/position/journal leakage, and no stale indicator/drawing state;
- keyboard input in trade/journal/drawing/indicator fields does not delete drawings or navigate replay; chart shortcuts work only outside editable/modal controls;
- honest loading/disabled/validation/backend-error recovery.

Retain machine-readable results and screenshots at minimum:

- 1440×1000 integrated workspace with chart, current context, open position/P&L, pending/recent order and journal/checklist accessible;
- 1440×1000 pre-T+2 error and later settled/executed state;
- 1280×800 integrated compact workspace with chart and trade/journal access visibly contained;
- reload/resume state with markers, position/order and journal/checklist restored.

Manually review every required screenshot. Require `runtimeErrors: []`, `providerErrors: []`, `indicatorRequestFailures: []`, no page/console error, and no blocking failure.

## Exact verification and final audit

Run at minimum:

```bash
git diff --check
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

Also run focused backend/frontend commands recorded in the ExecPlan and compare the final UAT IDs with the accepted Batch 3 baseline `test-results/product-uat/2026-07-18T06-24-40-732Z/results.json`. Report missing/duplicate/changed-pass/changed-blocking IDs explicitly.

Before handoff, review the diff against the ExecPlan and T-01–T-05/R-02/R-04/G-01–G-05, record deviations and accepted limitations, confirm no unrelated file is claimed, verify the production DB hash and protected tag, and update the ExecPlan progress/decision/evidence sections. Do not self-approve.

## Stop and escalation conditions

Stop and return to the Reviewer if:

- correct rewind/resume semantics require an unapproved migration, destructive production-record rewrite, event-sourcing redesign or broad accounting/backtesting rewrite;
- the existing contract cannot represent journal/checklist ownership without a versioning/architecture decision;
- a required chart/trade layout cannot preserve the accepted chart/pane geometry at the required viewports;
- a dependency, chart/provider change, private API, telemetry, external data transfer, acceptance weakening, production DB mutation or Batch 5 work would be required;
- any accepted Batch 1–3 regression remains unresolved.

Rollback must be bounded to Batch 4 composition, domain/service synchronization and additive UAT. Preserve all approved Replay/Indicator/Drawing state and existing opaque records. Do not perform rollback commands that discard the dirty checkout.

## Handoff

Stop at the Reviewer gate with:

- T-01–T-05, R-02, R-04 and G-01–G-05 status individually;
- exact files and contracts changed, focused/full test counts and commands;
- final UAT path, added assertion IDs, baseline comparison and screenshot review;
- exact forward/rewind/reload state evidence, T+2/order/position/journal evidence and error arrays;
- DB hash before/after, HEAD/tag provenance, deviations, rollback and known limitations;
- explicit confirmation that no branch/worktree/stage/commit/push/merge/reset/clean/tag/Batch 5 action occurred.

Final line:

`BATCH 4 DEV COMPLETE — STOPPED AT REVIEWER GATE`
