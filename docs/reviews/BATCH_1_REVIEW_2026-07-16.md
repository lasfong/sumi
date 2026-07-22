# Batch 1 Reviewer report — 2026-07-16

## Verdict

**RETURN TO DEV FOR BOUNDED HARDENING. Batch 2 is not authorized.**

The Horizontal Line vertical slice is real: it renders through a Lightweight Charts v5 series primitive, supports create/select/drag/price edit/delete, survives reload in the tested local repository, and the reviewer reproduced the deterministic browser run with zero console/page errors. The production database hash remained unchanged.

Batch 1 is not complete because the architecture extraction is largely a relocation of the old Replay monolith and several scoped UAT checks do not prove the acceptance requirement they claim. This is exactly the distinction between “technically works” and a reliable product foundation.

## Independently verified

- Branch/HEAD remained `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`.
- `git diff --check`: PASS.
- Backend tests: 75 passed, 1 skipped.
- Frontend tests: 11 files / 24 tests passed; lint and production build passed.
- Reviewer rerun: `test-results/product-uat/2026-07-16T13-42-35-488Z/results.json` — 29 overall passes, 13 retained product-gap failures, zero runtime errors, and zero failures under the current scoped classifier.
- All retained screenshots have the required dimensions and complete opaque pixel bounds. The apparent black/cropped rendering seen while loading multiple images in the reviewer UI was a viewer artifact, not a corrupted Playwright screenshot.
- `backend/sumi.db` SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Neither community drawing provider nor any new production dependency was added.

## Blocking findings

### B1-R01 — P1 — Replay architecture extraction is superficial

`ReplayPage.tsx` is now five lines, but `ReplayWorkspace.tsx` is also only five lines and delegates to a 593-line `ReplayWorkspaceController.tsx`. That controller still owns TanStack queries, WebSocket behavior, replay mutations, indicator orchestration, data transforms, keyboard handling, toolbar/header/chart/detail-panel JSX, and inline layout styling.

This moves the old monolith to a new filename; it does not establish the planned controller/view boundary. The Batch 1 ExecPlan claims that `ReplayWorkspace` is the composition shell and the controller owns state, but the implementation has the opposite practical shape: the controller renders the whole page and `ReplayWorkspace` is an empty wrapper.

Required closure: separate controller/view-model ownership from workspace composition. The controller layer must not render the full Replay UI. Keep the refactor bounded; do not redesign visuals or split every small element into a component.

### B1-R02 — P1 — D-06 is reported PASS without testing undo/redo for create, move, and edit

The browser check named `batch1.undo-create-move-edit` only presses Undo immediately after keyboard delete and verifies that the drawing returns. It does not undo or redo create, drag/move, or exact-price edit. The single history unit test only proves one generic before/after command.

Required closure: independently verify exact domain state and revision behavior for undo and redo of create, one drag, exact-price edit, UI/keyboard delete, and clear. One drag must remain one command.

### B1-R03 — P1 — Cancel is not wired to provider interaction and cannot safely cancel an active drag

Escape and Cursor call controller `setTool('select')`; they do not call `DrawingProvider.cancel()`. `SumiPrimitiveDrawingProvider.cancel()` is currently unused. If it were called during a drag, it clears the drag reference but does not restore the previewed document or release pointer capture. The current UAT only cancels before a one-click Horizontal creation, so it cannot expose this issue.

Required closure: define and test cancel semantics before creation and during an active drag. Cancel must restore the pre-drag drawing, release pointer capture, restore chart scrolling, leave persistence/revision/history unchanged, and visibly return to Select.

### B1-R04 — P1 — Revision conflict during undo/redo can corrupt history and throw through the UI

Normal persistence catches `DrawingRevisionConflict`, but `undo()` and `redo()` call `repository.save()` without error handling. `history.undo()`/`redo()` mutate the stacks before the save succeeds. A stale localStorage revision can therefore throw and leave command history inconsistent with the persisted document.

Required closure: make command application transactional or restore history on save failure; surface a controlled user message; add tests for stale revision during undo and redo.

### B1-R05 — P1 — Price-pane input boundary is not proved

The provider listens on the entire chart container and converts `clientY - container.top` through the candlestick series. The chart contains price, volume, RSI, MACD, and CCI panes. There is no explicit official-pane bounds check before create/select/drag. The current browser test clicks only the price pane.

Required closure: using official Lightweight Charts pane APIs only, reject pointer input outside the price pane. Add browser coverage proving that clicking Volume/RSI/MACD/CCI while Horizontal is active creates no drawing. If official APIs cannot make this deterministic, trigger the documented stop condition and open a base-chart-library reassessment ADR.

### B1-R06 — P1 — D-07 machine check proves domain count, not viewport attachment

`batch1.pan-zoom-replay` only checks that the drawing count is unchanged. It does not prove that the primitive remains visible at the correct price or can still be selected/moved after pan, zoom, resize, and replay advance.

Required closure: after viewport/replay operations, prove visible attachment and successful hit/select/move at the same Sumi-domain price semantics. Retain screenshots, but do not use screenshots alone as the machine assertion.

## Serious non-blocking architecture findings to close in the same hardening pass

### B1-R07 — P2 — Provider contract drift is undocumented

The Batch 0 approved contract includes selection snapshot, update/remove/clear/snapshot and error semantics. The implemented interface contains only `setTool`, `cancel`, `select`, `replaceDocument`, `subscribe`, and `destroy`. Some omitted mutations may correctly belong only to the domain controller, but that is an architecture change and must be reconciled in the decision record instead of silently narrowing the contract.

Do not add unused methods merely to match a checklist. Record the final ownership contract and add contract tests for the methods/events actually approved.

### B1-R08 — P2 — ChartWorkspace still contains two drawing input systems

`ChartWorkspace` creates the new primitive provider but also retains the legacy `activeTool`/`onDrawingComplete` pending-drawing click and crosshair subscriptions. Production currently leaves the legacy path inactive, but the facade still owns two interaction systems.

Preserve legacy record rendering as read-only compatibility. Remove or isolate the legacy input path so there is one production drawing interaction owner.

### B1-R09 — P2 — Provider unit coverage is too shallow for the claimed lifecycle foundation

The provider unit suite has one test covering create plus idempotent `destroy()`. It does not prove listener removal, selection/hit testing, one-commit drag, cancel rollback, pointer capture release, scroll restoration, or absence of events after destroy.

Add focused contract tests; do not rely exclusively on the browser happy path.

## Accepted limitations

- Local schema-v1 persistence instead of backend synchronization is accepted for Batch 1.
- Legacy backend drawing records may remain read-only and must not be migrated in this hardening pass.
- Trendline, Ray, Rectangle, Fibonacci, Text, magnet/snapping, and Indicator Manager remain out of scope.
- The 13 retained full-product failures remain expected and must not be removed, weakened, or reclassified as product success.

## Reviewer decision

Batch 1 functional direction is accepted, but completion is rejected until B1-R01 through B1-R06 pass with code and browser evidence. B1-R07 through B1-R09 must be resolved or explicitly accepted in the updated decision log. No additional drawing tool and no Batch 2 work may begin.

## Reviewer re-inspection — 2026-07-16

Status: **MOST HARDENING ACCEPTED; TWO FINAL CLOSURE ITEMS REMAIN. Batch 2 is still not authorized.**

The reviewer accepted the implementation and evidence for B1-R01, B1-R02, B1-R04, B1-R05, B1-R06, B1-R07, B1-R08, and the tested portions of B1-R09. The controller/view boundary is now real; command history changes only after CAS success; official price-pane bounds are used; legacy interaction ownership is removed; independent semantic undo/redo and viewport/replay interaction assertions pass.

Independent rerun evidence:

- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 12 files/31 tests; lint/build PASS.
- `test-results/product-uat/2026-07-16T14-14-04-019Z/results.json`: 41 pass / 13 retained product gaps / zero scoped failures / zero runtime errors.
- Production DB SHA-256 unchanged: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

### B1-R10 — P1 — Native `pointercancel` commits instead of cancelling

`SumiPrimitiveDrawingProvider` registers `pointercancel` with `onPointerUp`. During an active drag, that handler emits `change-committed`, advances persistence/history through the controller, and treats a browser/system cancellation as a completed user edit. Escape/Cursor cancellation is fixed, but native pointer cancellation is not.

Required closure: a dedicated pointer-cancel path must rollback the preview with the same transaction guarantees as explicit cancel, safely release capture if still held, restore scroll, emit no commit, and leave revision/persistence/history unchanged. Add a focused provider test and a scoped browser assertion using a real dispatched `pointercancel` during drag.

### B1-R11 — P1 — Session/symbol collision remains unwritable

`DrawingRepository.load(sessionId, newSymbol)` correctly returns null when the stored document has another symbol. The controller then starts a revision-0 document, but `save()` reads the old same-session key and compares against its nonzero revision. Every create/edit for the new symbol conflicts permanently. The added test proves only non-loading; it does not prove that the new identity can initialize and persist.

Required closure: define an explicit, non-destructive identity-collision policy and test the full load/initialize/save flow. A mismatched old document must not silently appear under the new symbol, and the new symbol must be able to persist without asking the user to manually clear all localStorage. Preserve or back up old state if the policy replaces a legacy key. Record compatibility consequences in the ExecPlan.

No other Batch 1 scope is reopened. After B1-R10/R11 pass, Reviewer may close Batch 1 and prepare the Batch 2 prompt.

## Final Reviewer closure — 2026-07-16

Status: **APPROVED AND CLOSED. Batch 2 is authorized to begin under its dedicated DEV prompt.**

The Reviewer inspected the final B1-R10/B1-R11 implementation and accepts both closure items:

- **B1-R10 closed:** native `pointercancel` has a dedicated rollback path. It restores the exact pre-drag drawing/selection, emits no commit, does not advance history/revision/persistence, restores scrolling and pointer-capture state safely, removes its listener on destroy, and is covered by focused provider tests plus the browser assertion `batch1.native-pointercancel-transaction`.
- **B1-R11 closed:** the drawing repository now keys canonical documents by session and encoded symbol, prevents cross-symbol loading, permits a new identity to initialize at revision 0, preserves mismatched legacy content under an identity/backup key, promotes same-symbol legacy content without data loss, and retains same-identity CAS conflict behavior.

Independent final gate evidence:

- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 12 files/34 tests; lint/build PASS.
- `./scripts/run-product-uat.sh`: PASS; independent result `test-results/product-uat/2026-07-16T14-37-46-450Z/results.json` records 42 passes, 13 retained out-of-scope product gaps, `blockingFailed: 0`, and zero runtime errors.
- Reviewer inspected the independent 1440×1000 and 1280×800 artifacts; Horizontal Line remains aligned and usable after pan/zoom/replay and the compact layout does not break the Batch 1 capability.
- Production DB SHA-256 after the independent UAT remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

This approval is deliberately narrow. It closes the Batch 1 replay-workspace foundation and Horizontal Line vertical slice. It does **not** approve the remaining drawing tools, Indicator Manager, legacy backend drawing migration, or the product as professionally usable. The 13 retained product gaps remain release blockers and must stay visible while later batches close them.
