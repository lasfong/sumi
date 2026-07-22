# DEV prompt — Batch 1 Reviewer hardening

You are the DEV owner returning to Sumi V3 Batch 1 after Reviewer rejection.

Work directly in `/Users/mizuhara/workspace/sumi` on the current checkout. Do not create/switch a branch or worktree. Do not commit, push, merge, reset, clean, retag, or discard any existing Reviewer/DEV changes.

## Authority and required reading

Read completely before editing:

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
5. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
6. `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`
7. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`
8. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`

The Reviewer report is authoritative for this hardening pass. Update the existing Batch 1 ExecPlan before code changes with the returned-gate status, affected modules, milestones, acceptance mapping, rollback, and exact verification commands.

## Outcome

Close B1-R01 through B1-R09 without expanding Batch 1. Preserve the working Horizontal Line slice while turning the claimed architecture and UAT contracts into real evidence.

## Required implementation work

### 1. Establish a real Replay controller/view boundary

- `ReplayPage` remains route composition only.
- Extract Replay orchestration into a controller/view-model hook or an equivalent non-visual controller boundary.
- `ReplayWorkspace` becomes the actual UI composition surface.
- The controller must not render the full header/chart/details JSX.
- Keep the refactor bounded: reuse the current header/chart/detail structures and do not perform a visual redesign.
- Preserve queries, WebSocket, replay navigation, indicators, markers, trade controls, journal, and resume behavior.

### 2. Make drawing command application transactional

- Undo/redo persistence conflicts must not throw through React handlers.
- A failed save must not mutate history stacks or diverge UI/domain/persistence state.
- Provide controlled conflict feedback.
- Add focused tests for stale revisions during ordinary commit, undo, and redo.

### 3. Implement real cancel semantics

- Escape and Cursor must invoke the provider/controller cancel path.
- Cancel before create produces no drawing.
- Cancel during drag restores the exact pre-drag price and selection semantics.
- Cancel releases pointer capture, restores chart scrolling, creates no history command, performs no persistence write, and leaves revision unchanged.

### 4. Enforce the price-pane boundary

- Use official Lightweight Charts v5 pane APIs only.
- Do not inspect or patch chart-library internals.
- Horizontal create/select/drag must accept input only inside the price pane.
- Clicking Volume, RSI, MACD, or CCI while Horizontal is active must create nothing.
- If official pane APIs cannot provide deterministic bounds, stop and document the architecture stop condition. Do not create a DOM-internals workaround.

### 5. Remove dual interaction ownership

- Preserve legacy backend drawings as read-only overlays.
- Remove or isolate the legacy `activeTool`/pending-click interaction path from production `ChartWorkspace`.
- The Sumi primitive provider must be the only production drawing interaction owner.

### 6. Reconcile the provider contract

- Compare the implemented interface with the approved Batch 0 contract.
- Decide explicitly which operations belong to `DrawingProvider` and which belong exclusively to the domain controller.
- Record the decision and consequences in the Batch 1 ExecPlan/decision log.
- Do not add dead methods just to mirror the old interface.
- Add focused contract tests for the final approved lifecycle/events.

## UAT corrections required

Replace the misleading `batch1.undo-create-move-edit` check with independent checks:

- undo create removes the exact created ID;
- redo create restores the same semantic drawing;
- undo move restores the exact prior price;
- redo move restores the moved price;
- undo exact-price edit restores the pre-edit price;
- redo edit restores the edited price;
- undo/redo UI delete;
- undo/redo keyboard delete;
- undo/redo clear;
- one drag increments revision exactly once;
- cancel during drag changes neither revision nor persisted document.

Strengthen viewport/lifecycle checks:

- after pan and zoom, the line remains visibly attached and can be hit/selected/moved;
- after replay advance and rewind, the same drawing ID remains visible and interactive without duplication;
- after resize at 1440×1000 and 1280×800, the line remains aligned with its domain price;
- ten mount/unmount cycles produce no duplicate listeners or duplicate provider events;
- clicking non-price panes while Horizontal is active creates no drawing;
- all checks retain machine-readable state, screenshots, and console/page errors.

Do not remove or weaken any existing product assertion. The 13 out-of-scope product failures remain visible. `blockingFailed` may remain a scoped gate, but every Batch 1 acceptance claim must have a real scoped assertion.

## Focused test requirements

Add provider/controller tests for:

- listener registration/removal and no events after destroy;
- idempotent destroy;
- selection/hit testing;
- one drag = one commit;
- cancel rollback and pointer/scroll cleanup;
- price-pane boundary;
- revision conflict during commit/undo/redo;
- history state preserved after failed persistence;
- session-ID/symbol collision behavior in local storage.

## Verification

Run at minimum:

```bash
git diff --check
cd frontend && npm test -- --run src/features/drawings src/components/chart src/components/replay
cd frontend && npm run lint
cd frontend && npm run build
cd backend && ../.venv/bin/python -m pytest app/tests/test_replay_no_future_leak.py app/tests/test_api_integration.py -q
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

Browser UAT must use the isolated temporary database and retain 1440×1000 plus 1280×800 evidence. Production DB SHA-256 must remain `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

## Prohibited scope

- No Batch 2.
- No Indicator Manager work.
- No Trendline, Ray, Rectangle, Fibonacci, Text, or magnet/snapping.
- No community drawing provider or new chart/drawing dependency.
- No backend drawing migration or rewriting legacy records.
- No visual redesign or cosmetic cleanup unrelated to reviewer findings.
- No weakening tests or acceptance criteria.

## Stop conditions

Stop and return evidence to Reviewer if official APIs cannot provide reliable pane bounds/hit testing, if Lightweight Charts internals must be patched, if a new dependency/chart library is required, or if closure materially exceeds the bounded Batch 1 hardening scope.

## Final handoff

End with:

```text
BATCH 1 HARDENING COMPLETE — STOPPED AT REVIEWER GATE

Reviewer findings closed:
- B1-R01: ...
...
- B1-R09: ...

Architecture:
- ...

UAT corrections:
- ...

Verification:
- ...

Evidence:
- ...

Production DB before/after:
- ...

Remaining limitations:
- ...

Deviations:
- ...
```

Do not start Batch 2. Stop for Reviewer inspection.
