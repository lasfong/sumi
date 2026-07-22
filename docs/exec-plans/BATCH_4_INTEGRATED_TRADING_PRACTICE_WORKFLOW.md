# Batch 4 — Integrated Trading-Practice Workflow

## Outcome

Deliver one coherent replay practice workspace in which current market context, the chart, indicators, drawings, trade actions, projected order/position/P&L/T+2 state, decision history, and an in-workspace journal/checklist stay synchronized through forward, rewind, reload, and resume. The chart remains primary at 1440×1000 and 1280×800. Batch 4 stops at the Reviewer gate and makes no product-complete, release-ready, or “TradingView-like” claim.

## Context and problem

- Authority: `docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md`; acceptance: T-01–T-05, R-02, R-04, and G-01–G-05.
- Reviewer-approved blocking baselines are Batch 1 Replay, Batch 2 Indicator Manager, and Batch 3 Drawing MVP. Accepted machine baseline: `test-results/product-uat/2026-07-18T06-24-40-732Z/results.json`, 224/224 passing with zero blocking/runtime/provider/indicator-request failures.
- Current Replay shows symbol/date/current visible count and OHLC but omits Volume and total session bars. Trade and decision panels are stacked below the drawing inspector in a continuously scrolling right rail; `/journal` is a separate route and there is no in-workspace checklist.
- Trade submission closes its modal even after controller-caught rejection, has no local validation/error state or duplicate-submit guard, and labels outcomes through transient toasts rather than a durable workflow state.
- Current decision markers are built from every trading action, so an unfilled LIMIT decision can look executed. Orders show only pending/rejected, positions omit current price/realized P&L/T+2 availability, and the controller independently refetches candles/position/orders/decisions, allowing stale mixed snapshots.
- Demonstrated contract defect: `ReplayService.previous_candle()` decrements only `current_index`. Stored future decisions/executions/orders/positions/trades/cash remain visible through existing endpoints. The current test explicitly preserves a decision beyond the rewound bar. This violates R-04/T-04 and requires a scoped service-layer projection, not route logic or a database rewrite.

## Provenance and preserved workspace

- Direct dirty checkout only: `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; protected `v2.0.0-rc2^{}` is `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Production `backend/sumi.db` SHA-256 before Batch 4: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Preserve the complete tracked/untracked inventory reported by `git status --short`, including all Reviewer/governance/Batch 1–3 code, tests, plans, prompts, decision packs, spikes, UAT scripts and artifacts. Do not claim unrelated dirty files as Batch 4 work.
- Installed production dependencies remain React, TanStack Query, Axios, Zustand, Lightweight Charts v5, lucide-react and react-hot-toast; dev-only AJV 8 remains the accepted Batch 3 validator. No Batch 4 dependency is planned.

## In scope

- A service-owned, read-only **practice snapshot as of `ReplaySession.current_index`** containing context/total bars, projected cash, positions, visible decisions, orders, executions/markers, trades, T+2 availability, latest lifecycle index, and historical-view eligibility.
- A narrow write guard preventing accounting/trade decisions while the replay cursor is earlier than retained future lifecycle activity; advancing to the latest activity restores write eligibility without deleting data.
- One coordinated frontend query/view model for lifecycle state, navigation synchronization, marker derivation from executions, honest validation/submission states, and duplicate-submit prevention.
- Sumi-owned integrated right-rail tabs/surfaces for Trade, Journal/Checklist, Decision Log and selected Drawing details without route loss or chart destruction.
- Journal/checklist ownership through the existing `/journal` table/contract: decision `reason`/`note` remain immutable submission rationale; `/journal` owns reflective notes and the versioned practice checklist.
- Additive backend/frontend tests and browser UAT for the complete deterministic practice path, all required viewports/states, and baseline assertion retention.

## Out of scope

- Batch 5, release work, a generic ledger/event-sourcing platform, alternate-timeline branching, broad accounting/backtesting rewrite, database migration, destructive record rewrite, cloud sync, telemetry, chart/provider/dependency changes, or mobile-first trading UX.
- Cancelling/amending orders, short selling, multiple simultaneous symbol positions, new execution algorithms, generic journal CRUD expansion, or reinterpretation of historical opaque records.
- Changes to backend indicator authority, no-future candle delivery, provider-independent drawing documents, or accepted fixed pane policy.

## Invariants

- The browser receives candles only through `current_index`; backend indicator calculation remains authoritative and drawings receive only visible candles.
- Historical ledger rows are never deleted or destructively rewritten by rewind. Projection is deterministic and read-only; forward restores retained facts without duplicate order matching or duplicate markers.
- FastAPI routes delegate business/projection/journal validation to services.
- Every trade/checklist action is tied visibly to the same session/symbol/date/bar context rendered in the header.
- UAT/backend tests use temporary databases; the production DB hash stays identical.
- All 224 accepted assertion IDs remain present exactly once, passing and blocking under their existing classifier.

## Current architecture

- `ReplayWorkspaceController.tsx` owns separate candles/session/position/decision/order queries, navigation invalidation, markers and trade calls; journal is absent.
- `ReplayWorkspace.tsx` renders header, indicator strip, drawing rail, chart and a single scrolling detail rail containing inspector, trade, orders, position and Decision Log.
- `TradeControls.tsx` contains the action modal but no synchronous validation, submit state, returned result, or context header.
- `backend/app/api/decisions.py` returns mutable current tables directly. `ReplayService.next_candle()` mutates MTM/matches pending orders; `previous_candle()` only moves the cursor.
- `/journal` stores `note_type`, `content`, optional association IDs and tags but has no service validation, replay-context semantics, or integrated UI.

## Target design and state ownership

```text
Replay route
  -> useReplayWorkspaceController
       -> candles + session context query
       -> practiceSnapshot query (one as-of lifecycle authority)
       -> journal query (as-of current context)
       -> indicator/drawing controllers (unchanged ownership)
       -> coordinated navigation refresh epoch
  -> ReplayWorkspace
       -> context/replay header
       -> indicator manager
       -> drawing rail + chart (primary)
       -> PracticeRail tabs
            Trade: actions, order, position/P&L/T+2
            Journal: checklist, notes, saved entries
            Decisions: intentional decision log
            Drawing: selected inspector when applicable

PracticeWorkflowService (backend)
  -> immutable decisions/orders/executions/trades + replay candles
  -> deterministic as-of projection, marker facts and write guard
JournalService
  -> association/session validation + checklist envelope validation
  -> as-of visibility for integrated entries
```

The practice snapshot is the only frontend authority for orders, positions, trades, executions/markers, projected cash and T+2. Existing endpoints remain compatibility surfaces; no second client-side accounting model is introduced. TanStack Query owns server state; local component state owns only drafts/tab/open state.

## Forward, rewind, reload and resume semantics

1. Ledger facts remain durable. Decisions and orders use their decision `candle_index`; executions map `execution_date` to the session candle index. Unknown legacy rows are handled conservatively and never fabricated.
2. At index `i`, visible decisions/orders require decision index `<= i`; visible executions require execution index `<= i`. Markers are emitted only from visible executions, so HOLD/SKIP and unfilled LIMIT decisions never imply a fill.
3. Projected cash starts at `initial_cash` and applies visible execution `net_amount` by order side. Position quantity/average/realized/unrealized P&L and projected trades are rebuilt from visible executions. T+2 available quantity is visible settled BUY quantity whose decision index is `<= i - 2`, minus visible SELL quantity.
4. A stored executed LIMIT whose fill is beyond `i` projects as pending; at its execution index it projects as executed. Existing stored execution prevents `_match_pending_orders` from creating a duplicate when the cursor later crosses that bar again.
5. Rewind changes only the cursor and therefore the projection. It hides future decisions/orders/fills/positions/trades/markers and context-bound journal/checklists without deleting them. Indicators/drawings continue their accepted current-index behavior.
6. If any retained lifecycle activity exists beyond the cursor, snapshot status is `historical` and new trading decisions are disabled/rejected with an actionable “advance to latest activity” explanation. This avoids unapproved alternate-timeline accounting. Journal observations may be recorded at the viewed context because they do not mutate accounting.
7. Forward, ±5 and autoplay refresh candles, session, practice snapshot and journal as one observed navigation epoch. Existing retained fills reappear rather than re-execute. New genuinely pending orders may match only once on newly visited bars.
8. Reload/resume restores the backend replay index and projected snapshot, local indicator document, local/backend drawing document and current visible journal/checklist. No client-only reconstruction is canonical.

## Journal and checklist contract

- `Decision.reason` and `Decision.note` remain rationale/review text captured with a decision; they are not a mutable journal authority.
- `/journal` is the canonical reflective journal/checklist authority. Existing legacy note types remain readable.
- Batch 4 checklist entries use `note_type = "practice_checklist"` and a versioned JSON envelope in existing `content`: `schemaVersion`, exact `context` (`sessionId`, `symbol`, `candleIndex`, `date`), explicit booleans (`trendIdentified`, `setupConfirmed`, `entryTriggerDefined`, `riskDefined`, `exitPlanDefined`, `emotionChecked`) and `observation`. This uses the existing opaque text contract and needs no migration.
- `JournalService` validates nonblank bounded content, session ownership, optional decision/trade association, checklist keys/types/context, and filters integrated context-bound entries after rewind. Unlinked legacy entries have no historical index and remain session-level compatibility notes; the UI labels them as such rather than pretending they are bar-bound.

## Layout, responsive and keyboard policy

- At 1440×1000, preserve the accepted chart/pane minimum-height and vertical-scroll policy. Use a labelled tabbed right rail rather than stacking every workflow panel; Trade is default, Journal/Decisions are one click away, and Drawing opens automatically when a drawing is selected without covering chart/replay controls.
- At 1280×800, keep the same contained rail with internal scroll and compact tab labels. The indicator strip retains its accepted horizontal overflow. Chart, replay navigation and Trade/Journal access must remain visible and understandable; no core control may overlap.
- Minimum supported full workstation width is 1180px. Below it, Sumi shows an explicit limited-desktop/mobile message and disables trade submission; chart review/navigation may remain available. Phone-optimized professional trading is not claimed.
- Editable/modal controls stop propagation for Space, arrows, Delete/Backspace and Escape as appropriate. Global replay/drawing shortcuts act only when focus is outside input/textarea/select/button/dialog content. Escape closes the topmost trade/journal draft before affecting drawing/replay state.

## Validation, loading, disabled and error rules

- BUY/SELL require positive finite quantity; LIMIT requires positive price; optional stop/target must be positive and coherent with action/current price. HOLD/SKIP ignore order quantity/type and never create orders/markers. CLOSE requires projected open quantity. Historical mode, navigation, session loading and active submission disable trade actions.
- A modal remains open on validation, backend rejection or transport error. One in-flight request per submission; controls show `Submitting…`; exact rejection/transport text is durable in the panel and recoverable without duplicate POST.
- Pending LIMIT rows explain that future candle range can fill them. Executed/cancelled/rejected rows appear in recent orders with distinct status, never under a misleading pending-only label.
- Position UI shows symbol, quantity, average/current price, realized/unrealized P&L, projected cash and T+2 available quantity. Pre-T+2 rejection leaves snapshot byte-equivalent and displays the exact constraint.
- Journal shows loading, empty, saving, saved and transport/error states; drafts survive a failed save and close/reopen does not destroy chart/session state.

## Expected affected modules

- Backend: add `backend/app/services/practice_workflow_service.py` and schemas; refactor decision read routes to delegate or add one `/practice-state` endpoint; add `JournalService` and route delegation; add a narrow historical-write guard in `TradeLifecycleService`; add focused tests in a new workflow test file and journal/API tests.
- Frontend domain/API: extend `decisionApi.ts`, `journalApi.ts`, trade/journal types; add a pure integrated workflow view-model/validation module and tests.
- Frontend UI: refactor `ReplayWorkspaceController.tsx`, `ReplayWorkspace.tsx`, `TradeControls.tsx`, `PendingOrdersPanel.tsx`, `PositionPanel.tsx`, `DecisionJournal.tsx`; add focused `PracticeRail` and `PracticeJournal` components/tests where ownership is clearer.
- Harness: extend `scripts/product-uat.mjs` additively and retain new required screenshots/results/logs.

## Milestones

1. **Red defect proof and projection domain:** focused backend test demonstrates future lifecycle leakage after rewind, then passes with deterministic as-of cash/order/position/trade/T+2 projection and historical write guard.
2. **Journal contract:** service validation/association/context visibility plus versioned checklist envelope and frontend parser/serializer tests pass without migration.
3. **Integrated controller:** one practice snapshot authority, execution-based markers and coordinated navigation/reload requests pass focused race/deduplication tests.
4. **Vertical UI:** context-rich action modal, honest state feedback, trade/order/position panels and in-workspace journal/checklist operate through the responsive tabbed rail with keyboard isolation.
5. **Browser workflow:** BUY/SELL/HOLD/SKIP/LIMIT, fill/rejection/T+2, journal/checklist, rewind/forward/autoplay/±5/reload/resume and retained indicators/drawings are proven with exact machine state and screenshots.
6. **Final gates:** baseline comparison, complete verification, manual artifact review, DB/tag/diff audit, ExecPlan evidence update and stop at Reviewer gate.

## Acceptance mapping

| ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| T-01 | Tabbed responsive PracticeRail; retained chart minimum/scroll policy | 1440×1000 and 1280×800 geometry/screenshots |
| T-02 | Validated/deduplicated actions; projected orders/position/P&L/T+2 and durable states | Real BUY/SELL/HOLD/SKIP/LIMIT/fill/rejection paths |
| T-03 | Journal/checklist tab, versioned existing-contract envelope, explicit ownership | create/associate/close/reopen/rewind/reload browser proof |
| T-04 | Service as-of projection, execution markers, one snapshot query | boundary rewind/forward/reload exact equality/no duplicates |
| T-05 | Complete deterministic practice-session sequence | additive end-to-end browser scenario and no runtime errors |
| R-02 | symbol/date/bar-current/total/OHLCV in header and inherited panel context | exact DOM text at both viewports |
| R-04 | coordinated navigation epoch and projected lifecycle state | step/±5/autoplay/pause and request/state assertions |
| G-01 | focused/full backend/frontend/lint/build/UAT/product gates | exact commands and counts |
| G-02 | temporary UAT DB only | runtime path plus unchanged production hash |
| G-03 | retained results/screenshots/logs for every run | timestamped artifact directories |
| G-04 | no unresolved Batch 4 P0/P1 and no accepted regression | diff/acceptance/ID self-review |
| G-05 | local APIs/local persistence only; no telemetry/dependency | source/network audit |

## Verification commands

```bash
git diff --check
cd backend && ../.venv/bin/python -m pytest app/tests/test_practice_workflow.py app/tests/test_trade_lifecycle.py app/tests/test_api_integration.py -q
cd frontend && npm test -- --run src/features/practice src/components/replay
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

## Rollback and compatibility

- Projection and `/practice-state` are additive read contracts; legacy decision/order/position/trade/journal rows remain untouched. Rollback restores old controller queries/panels and removes the additive endpoint/services without a database rollback.
- Existing decision and journal endpoints remain readable for the separate Journal page and compatibility. New checklist JSON remains valid opaque journal content even if the integrated UI is rolled back.
- Historical write guard can be removed with the projection service, returning to the old behavior without rewriting records. No source rollback command will be used on the shared dirty checkout.

## Risks and mitigations

- **Projection differs from mutable accounting:** reuse `Execution.net_amount`, order side and recorded decision indices; compare current-tip projection against stored session/position/trade state in focused tests.
- **Execution date/index ambiguity:** map within the exact session candle series/timeframe and stop if a retained execution cannot map deterministically; do not guess or migrate.
- **Alternate timeline after rewind:** explicit historical read-only mode until latest retained activity, rather than destructive truncation or branching.
- **Journal opaque JSON drift:** strict versioned serializer/parser and backend validation; legacy notes remain labelled compatibility data.
- **Navigation races/duplicate requests:** one snapshot query key plus coordinated awaited refresh and request-count UAT.
- **Layout pressure:** tabs/internal scroll, accepted chart minimum and two viewport gates; stop rather than shrink panes below policy.

## Stop conditions

- Stop for Reviewer if deterministic projection requires a database migration, destructive record rewrite, generic event sourcing, ambiguous execution-to-candle mapping, unresolved multi-position accounting semantics, or a new versioning/ownership decision.
- Stop if the existing journal text contract cannot safely carry the versioned checklist, accepted chart geometry cannot survive the integrated rail, or any Batch 1–3 regression remains.
- Stop before any dependency/chart/provider/private API/telemetry/external transmission/acceptance weakening/production DB/Batch 5 action.

## Progress log

- 2026-07-18: read the complete standalone prompt and all 13 mandatory authority/review/ExecPlan sources in order; confirmed Batch 3 final Reviewer approval and Batch 4 authorization.
- 2026-07-18: recorded branch/HEAD/tag, dirty inventory, dependency state, accepted 224-ID UAT baseline and production DB hash.
- 2026-07-18: audited current 1440×1000 and 1280×800 browser artifacts plus Replay/trade/order/position/decision/journal UI, API, model, schema, service and test contracts. Confirmed the rewind projection defect and current stacked-rail/context/validation/marker gaps.
- 2026-07-18: chose a non-destructive as-of projection plus historical write guard as the narrow service correction; recorded exact ownership, navigation, journal, layout, error, rollback, test and stop semantics before product code.
- 2026-07-18: added `PracticeWorkflowService` and its additive snapshot schema/endpoint, historical write guard, `JournalService`, strict versioned checklist validation and six focused backend scenarios. The initial red test failed on the absent projection module before implementation.
- 2026-07-18: integrated one lifecycle snapshot authority into Replay, execution-only markers, validated/deduplicated durable trade feedback, projected order/position/P&L/T+2 panels, tabbed PracticeRail and in-workspace Journal/checklist. No dependency or migration was added.
- 2026-07-18: extended browser UAT additively with 22 blocking `batch4.*` checks. A first regression run exposed `batch1.compact-layout`; rail width was reduced within the planned range and the accepted assertion was restored without changing its criterion. A later image review exposed the mobile warning stylesheet in an unused file; the rule was moved to the active stylesheet and trade actions now use true viewport-driven `disabled` state.
- 2026-07-18: standalone UAT `2026-07-18T07-21-14-202Z` passed 246/246. Final full product gate UAT `2026-07-18T07-22-47-858Z` also passed 246/246 with zero blocking failures. Required 1440×1000 and 1280×800 screenshots were manually reviewed; the 1440 warning is absent, chart remains primary, rail content is contained, T+2 rejection is legible, and journal/reload evidence is visible.
- 2026-07-18: completed focused/full tests, lint, build, `verify-v2`, standalone UAT, `verify-product`, diff, baseline-ID, DB hash, HEAD and protected-tag audits. Stopped at Reviewer gate without self-approval.

## Decision log

- Preserve ledger facts and project them as-of the cursor instead of deleting future rows on rewind. Destructive truncation loses practice history; event sourcing/branching exceeds Batch 4.
- Use one practice snapshot query rather than independently fetched positions/orders/decisions/trades because atomic visible context is part of R-04/T-04.
- Derive chart trade markers from executions, not trading-action decisions. HOLD/SKIP remain intentional decisions; pending/rejected orders never imply execution.
- Use `/journal` as canonical checklist/reflection storage and keep `Decision.reason/note` as submission rationale. The existing opaque content field supports a strict versioned envelope without migration.
- Use a tabbed details rail to preserve chart dominance and keep journal reachable without route loss; do not add a drawer dependency or claim phone optimization.
- Classify the one browser console message caused by the deliberately exercised HTTP 400 T+2 rejection only inside that controlled request window and assert it exactly once as `batch4.G-05.expected-rejection-contained`; all other page/console errors remain blocking in `runtime.no-errors`.

## Completion evidence

- Acceptance status: T-01, T-02, T-03, T-04, T-05, R-02, R-04 and G-01 through G-05 have implementation, focused-test and browser evidence. This is a DEV handoff only; Batch 4 is not self-approved.
- Backend: focused workflow/lifecycle/API/no-future-leak suite passed 27 tests; full technical gate passed 81 tests with 1 skipped. Frontend: 20 files / 113 tests passed; lint and production build passed. `git diff --check`, `./scripts/verify-v2.sh`, `./scripts/run-product-uat.sh` and `./scripts/verify-product.sh` all passed.
- Final machine evidence: `test-results/product-uat/2026-07-18T07-22-47-858Z/results.json`, 246 passed, 0 failed, 0 blocking. Arrays: `runtimeErrors: []`, `providerErrors: []`, `indicatorRequestFailures: []`; the single intentionally generated T+2 resource error is separately retained and asserted in `expectedPracticeConsoleErrors`.
- Added blocking IDs: `batch4.T-01.integrated-wide-layout`, `batch4.R-02.exact-visible-context`, `batch4.G-02.validation-before-api`, `batch4.G-03.duplicate-submit-deduped`, `batch4.T-02.buy-position-pnl`, `batch4.T-03.hold-skip-no-order-or-marker`, `batch4.T-04.context-checklist-created`, `batch4.T-05.pre-t2-backend-rejection`, `batch4.G-05.expected-rejection-contained`, `batch4.T-06.limit-pending`, `batch4.T-07.limit-filled-on-visible-candle`, `batch4.T-08.t2-available-later`, `batch4.T-09.close-realized-pnl`, `batch4.R-04.rewind-hides-future-close`, `batch4.R-05.rewind-limit-projection`, `batch4.T-10.rewind-journal-context`, `batch4.R-06.forward-restores-exactly-once`, `batch4.T-11.forward-restores-journal`, `batch4.G-04.reload-resume-full-state`, `batch4.T-01.integrated-compact-1280x800`, `batch4.T-01.compact-full-workstation`, and `batch4.T-01.explicit-limited-mobile`.
- Accepted baseline comparison against `2026-07-18T06-24-40-732Z`: all 224 IDs are present exactly once; missing 0, duplicate 0, changed-pass 0, changed-blocking 0. No accepted ID was renamed, skipped, weakened or removed.
- Required screenshots reviewed in the final artifact: `16-batch4-integrated-position-pending-journal.png`, `17-batch4-t2-rejection.png`, `18-batch4-settled-closed.png`, `19-batch4-reload-resume.png`, and `20-batch4-compact-1280x800.png`. Runtime logs are retained at `/var/folders/cq/17wktb557fq87zb4pczl4lh00000gp/T/sumi-product-uat.gd9GPB`.
- Rewind/forward evidence: bar 66 rewind hides the SELL/CLOSE execution and restores the open position; rewind to bar 64 projects the retained LIMIT as pending and hides its future execution; forward restores the same decision/order/execution IDs exactly once. Journal at bar 66 hides at bar 64 while the bar-64 checklist remains, then restores on forward. Reload retains the exact execution IDs, both checklists, and byte-equivalent indicator/drawing documents.
- T+2 evidence: market BUY creates quantity 100 with zero availability; immediate SELL is rejected without any new decision/order/execution; LIMIT BUY projects pending then executes once on the next visible candle; at T+2 the projected available quantity is 200 and CLOSE produces one SELL execution and a closed trade.
- Production DB SHA-256 before/after is unchanged: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`. Final provenance remains `master` / HEAD `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2^{}` remains `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Deviations: no architecture, migration, dependency or acceptance deviation. The compact rail width changed from the initial implementation choice to the narrower planned bound after browser regression evidence. The explicit limited-workstation threshold is viewport width 1180px; below it chart review/navigation remain available but trade actions are disabled. Multi-position/alternate-timeline/mobile-first UX remain intentionally out of scope.
- Rollback remains the bounded removal of additive snapshot/journal services, PracticeRail composition, practice domain/types/tests and Batch 4 UAT checks; no database rollback or dirty-tree discard is required.

## Reviewer gate — 2026-07-18

Status: **RETURNED FOR ONE BOUNDED CLOSURE. Batch 4 remains open; Batch 5 is unauthorized.**

The Reviewer accepted the submitted artifact integrity, ordinary single-step workflow, responsive layout and existing green gates, but independently reproduced two P1 contract defects:

- **B4-R01:** multi-step `next_candle(..., steps=N)` jumps directly to the destination and matches pending LIMIT orders only against that candle. An intermediate-only eligible fill is skipped, so `+5` is not lifecycle-equivalent to five single steps.
- **B4-R02:** checklist validation binds session/symbol/index but accepts any nonempty `date`; a `2099-12-31` context was stored for an authoritative `2024-01-02` candle.
- **B4-R03:** the 246-ID UAT covers LIMIT fill only through repeated single steps and checks journal `candleIndex` without an adversarial date mismatch, so it cannot detect either defect.

Independent existing gates: focused backend 19 passed; full backend 81 passed / 1 skipped; frontend 20 files / 113 tests; lint/build, `git diff --check`, and `./scripts/verify-v2.sh` passed. The submitted 246 checks are unique and internally green, and required screenshots were reviewed. Production DB and protected tag remained unchanged.

Closure authority: `docs/dev-prompts/BATCH_4_REVIEW_CLOSURE_PROMPT.md`. Preserve all 246 baseline IDs, add exact blocking evidence, update this plan, and stop again at Reviewer gate. Do not begin Batch 5.

## Review closure scope — B4-R01–B4-R03

### Outcome and boundaries

- Correct only the two Reviewer-reproduced service defects and the missing additive evidence. Batch 4 remains open until the Reviewer accepts this closure; Batch 5, migrations, event sourcing, alternate timelines, order amendment/cancellation, dependencies and UI redesign remain out of scope.
- Preserve all durable decisions/orders/executions/trades and the non-destructive as-of projection. Multi-step forward traversal must process every crossed candle chronologically and stop immediately if bankruptcy occurs; it must never recreate an execution already retained in the ledger.
- Preserve checklist schemaVersion 1 and readable legacy opaque notes. The backend, not the client, binds a new practice checklist to the exact authoritative candle for the session symbol/timeframe/adjustment/date range/current index.
- Preserve all 246 checks from `test-results/product-uat/2026-07-18T07-22-47-858Z/results.json` exactly once with unchanged ID, pass and blocking classification. Closure checks are additive and blocking.

### Affected modules and acceptance mapping

| Finding | Modules | Acceptance | Closure evidence |
| --- | --- | --- | --- |
| B4-R01 | `backend/app/services/replay_service.py`, `backend/app/tests/test_practice_workflow.py` | T-02, T-04, T-05, R-04, G-04 | intermediate-only hit, destination/no-hit/earliest-hit/completion cases; multi-step vs repeated-step state equality; rewind/forward exact execution identity |
| B4-R02 | `backend/app/services/journal_service.py`, `backend/app/tests/test_practice_workflow.py`, API integration coverage where appropriate | T-03, R-02, G-04 | exact canonical current date accepted; wrong valid, malformed/impossible, stale index and cross-session/symbol contexts rejected with zero inserted rows |
| B4-R03 | `scripts/product-uat.mjs` | G-01–G-04 plus retained T/R assertions | visible `+5` fills an intermediate-only LIMIT at the earliest candle; exact order/execution/position/cash/T+2 and exactly-once rewind/forward/reload; mismatched date rejected without insert; correct checklist visible after reload |

### Target service semantics

1. Resolve the exact primary candle sequence once for `next_candle`. Validate positive steps and a nonempty session series. Compute the bounded destination index.
2. Advance one index at a time from the current index through the destination. For each crossed candle, persist the cursor, perform MTM/equity/bankruptcy handling for that candle, then match currently pending LIMIT orders against that candle. This is the same chronological state transition as repeated `steps=1` calls.
3. A retained execution makes its order non-pending, so rewind followed by multi-step forward only reveals the fact through projection and cannot execute it again. End-of-session status becomes completed only at the final candle unless bankruptcy terminates traversal first.
4. Checklist creation resolves the authoritative primary candle using the same exact session dimensions. The submitted `YYYY-MM-DD` value must parse as an actual calendar date and equal the canonical ISO date for `candles[current_index]`. Missing/ambiguous candle resolution is an error; no journal mutation occurs before validation completes.

### Red reproductions and exact verification

- First add focused failing tests for: an intermediate-only LIMIT missed by `steps=3`; multi-step/repeated-step lifecycle equality including cash/position/P&L/T+2; no-hit, destination-hit, earliest of multiple hits, completion boundary and rewind/forward exactly-once; and wrong-valid/impossible/malformed checklist dates with zero journal insertion. Record the failing assertions before implementation.
- Focused closure command: `cd backend && ../.venv/bin/python -m pytest app/tests/test_practice_workflow.py app/tests/test_trade_lifecycle.py app/tests/test_api_integration.py -q`.
- Full required commands remain: `git diff --check`; full backend pytest; full frontend tests/lint/build; `./scripts/verify-v2.sh`; `./scripts/run-product-uat.sh`; `./scripts/verify-product.sh`; production DB SHA-256; baseline-ID comparison and screenshot review at 1440×1000 and 1280×800.

### Rollback and compatibility

- Rollback is bounded to the chronological loop/helper extraction in `ReplayService`, authoritative date resolution in `JournalService`, added focused tests and additive closure UAT. No schema/data rollback is needed because no migration or stored-row rewrite is introduced.
- Existing opaque journal notes remain untouched and readable. Existing checklist rows are not rewritten; exact date binding applies only to creation. Existing execution IDs and order facts remain authoritative.

## Review closure progress — 2026-07-18

- Recorded the two Reviewer defects as red before implementation. `test_multistep_advance_fills_limit_on_earliest_intermediate_candle` failed with zero executions where one was required; `test_checklist_rejects_wrong_valid_date_without_inserting` failed because the wrong valid date was accepted. The combined red run reported 2 failed.
- B4-R01: changed `ReplayService.next_candle()` to validate the requested step count, bound the destination, and apply cursor persistence, MTM/equity/bankruptcy and pending-order matching to every crossed candle in chronological order. Added intermediate-only, no-hit, destination-hit, earliest-multiple-hit, completion-boundary, rewind/forward identity, multi-step/single-step lifecycle equality and bankruptcy equality coverage.
- B4-R02: changed `JournalService` to parse canonical ISO dates and compare them with the exact authoritative current candle from the retained session series before insertion. Added exact-date acceptance plus wrong-valid, malformed, impossible, noncanonical, stale-index, cross-session and cross-symbol zero-mutation coverage at service/API level.
- B4-R03: replaced the former repeated single-step LIMIT path with the visible `+5` control and an intermediate-only deterministic fixture. Added eight blocking closure IDs for earliest fill, exact price/order/execution/cash/position/T+2/marker state, rewind/forward/reload identity, adversarial date rejection/no insertion and correct-date persistence. `ReplayWorkspace` exposes the already-derived marker array only as hidden machine-readable UAT diagnostics; chart/provider behavior and user-visible layout are unchanged.
- Final standalone UAT `2026-07-18T15-13-08-006Z` and final full product-gate UAT `2026-07-18T15-14-47-935Z` each passed 254/254 with zero failed or blocking checks. The 246-ID accepted baseline is present exactly once with missing 0, duplicates 0, renamed 0, changed pass 0 and changed blocking classification 0; all eight additive closure IDs pass and are blocking.
- Reviewed final full-gate screenshots at 1440×1000 and 1280×800. The `+5` screenshot shows the LIMIT executed on bar 65 while the destination is bar 69; the compact workstation remains contained. The reload Journal screenshot retains the exact-date exit checklist and the earlier entry checklist.

## Review closure decisions and deviations

- Preserve the old single-step transition order exactly: mark-to-market/equity and bankruptcy handling precede pending LIMIT matching at each crossed candle. A bankruptcy stops traversal immediately; completion is assigned only at the true final candle unless bankruptcy supersedes it.
- Validate checklist dates at creation only. Retained opaque notes and retained schemaVersion 1 rows are not parsed more aggressively, migrated or rewritten.
- Use the seeded local FPT formula only to choose a not-yet-visible candidate price for deterministic UAT. Exact eligibility, earliest execution candle/date/price and destination non-eligibility are asserted against backend candles only after the visible `+5` action reveals them, preserving the no-future-data product invariant.
- Deviation from the initial closure affected-module list: `frontend/src/components/replay/ReplayWorkspace.tsx` gained one hidden serialized marker output so browser evidence can assert the actual marker state through rewind, forward and reload. This is additive diagnostic evidence, not a second marker authority or a visible product change.
- No architecture, database, migration, dependency, provider, acceptance or stored-data deviation. Known non-blocking command output is limited to the existing Starlette/httpx deprecation warning and Node localStorage experimental warnings.

## Review closure final evidence

- Focused backend: `cd backend && ../.venv/bin/python -m pytest app/tests/test_practice_workflow.py app/tests/test_trade_lifecycle.py app/tests/test_api_integration.py -q` — 40 passed, 1 warning.
- Full backend: `cd backend && ../.venv/bin/python -m pytest -q` — 97 passed, 1 skipped, 1 warning. Frontend: `npm test -- --run` — 20 files / 113 tests passed; `npm run lint` and `npm run build` passed.
- `git diff --check`, `./scripts/verify-v2.sh`, `./scripts/run-product-uat.sh` and `./scripts/verify-product.sh` passed on the final closure state. Final machine results: `test-results/product-uat/2026-07-18T15-14-47-935Z/results.json`; runtime logs: `/var/folders/cq/17wktb557fq87zb4pczl4lh00000gp/T/sumi-product-uat.8VC1PR`.
- Error arrays: `runtimeErrors: []`, `providerErrors: []`, `indicatorRequestFailures: []`. `expectedPracticeConsoleErrors` contains exactly the controlled T+2 HTTP 400 and mismatched-date HTTP 409 resource messages; each is bounded and asserted in its deliberate request window.
- Additive blocking IDs: `batch4.closure.R01.plus5-intermediate-fill-earliest`, `batch4.closure.R01.plus5-exact-lifecycle`, `batch4.closure.R02.mismatched-date-rejected-no-insert`, `batch4.closure.R03.expected-date-rejection-contained`, `batch4.closure.R02.authoritative-date-saves-and-reloads`, `batch4.closure.R01.rewind-plus5-restores-limit-once`, `batch4.closure.R01.reload-preserves-limit-once`, `batch4.closure.R02.authoritative-date-present-after-reload`.
- Final reviewed screenshots: `test-results/product-uat/2026-07-18T15-14-47-935Z/21-batch4-closure-plus5-intermediate-fill.png`, `19-batch4-reload-resume.png`, and `20-batch4-compact-1280x800.png`.
- Production DB SHA-256 remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`. Provenance remains `master` / HEAD `108aa5dc0e26994607836e2b3b33f482e3791b4e`; protected `v2.0.0-rc2^{}` remains `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Closure status is DEV handoff only. B4-R01–B4-R03 are implemented and evidenced, Batch 4 remains open pending independent Reviewer acceptance, and Batch 5 remains unauthorized.

## Final Reviewer closure — 2026-07-18

Status: **APPROVED AND CLOSED.**

The Reviewer independently confirmed B4-R01–B4-R03 and reran the complete gates. Focused backend passed 40; full backend passed 97 with 1 skipped; frontend passed 113 tests plus lint/build; `verify-v2.sh`, standalone UAT and `verify-product.sh` passed. Independent browser artifacts `2026-07-18T15-22-01-472Z` and `2026-07-18T15-23-28-222Z` each record 254/254, zero blocking failure and empty runtime/provider/indicator-request failure arrays. All 246 baseline IDs remain unchanged and exactly eight blocking closure IDs are additive.

Required wide/compact screenshots were inspected. Production DB SHA-256, HEAD and protected tag remained unchanged. No unresolved Batch 4 P0/P1 remains. Batch 4 is closed; Batch 5 is authorized only through `docs/dev-prompts/BATCH_5_PRODUCT_HARDENING_V3_RC_PROMPT.md` and must stop at its own Reviewer gate.
