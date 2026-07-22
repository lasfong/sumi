# DEV prompt — Batch 3 Professional Drawing MVP

Continue in the existing Sumi checkout and working tree. Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, retag, discard, or overwrite existing Reviewer/user changes.

This prompt authorizes **Batch 3 only**. Batch 1 and Batch 2 are Reviewer-approved and must remain green. Batch 4 trading-workflow work is not authorized.

## Read before editing

Read completely, in this order:

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
4. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
5. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
6. `docs/DEVELOPMENT_OPERATING_MODEL.md`
7. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
8. `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`
9. `docs/decision-packs/sumi-drawing-document-v1.schema.json`
10. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
11. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`, including final closure
12. Batch 1/2 ExecPlans and current drawing domain/repository/history/controller/provider/toolbar/UAT code and tests.

Before product code changes, create and maintain `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md` using `PLANS.md`. Record baseline, exact affected modules, D-01–D-11 mapping, tool interaction contracts, schema/migration/persistence policy, risks, rollback, milestones, exact commands, progress, decisions, deviations, and artifact paths.

## Outcome

Deliver the complete professional drawing MVP required by D-01–D-11 using the approved `SumiPrimitiveDrawingProvider` and official Lightweight Charts v5 primitives:

- Cursor/Select
- Horizontal Line
- Trendline
- Ray
- Rectangle
- Fibonacci Retracement
- Text/Note

Every tool must have a complete lifecycle: discoverable selection, obvious active state, deterministic creation/cancel, visible selection geometry, body move, anchor edit where applicable, UI/keyboard deletion, undo/redo, versioned persistence, reload restoration, pan/zoom/replay/resize stability, magnet behavior, and lifecycle cleanup.

Do not install or connect Deepentropy, Difurious, another drawing package, or another chart library. Do not patch Lightweight Charts internals.

## Tool interaction contract

Record the exact pointer/keyboard contract in the ExecPlan before implementation and keep it consistent across toolbar labels, UI help/tooltips, tests, and UAT.

Minimum required semantics:

- **Cursor/Select:** click selects the topmost unlocked drawing; click empty price-pane space deselects; selection never creates a drawing; Escape cancels active creation/drag/edit and returns to Select.
- **Horizontal Line:** one click creates one price line; body/handle vertical drag edits its price.
- **Trendline:** two anchors create a finite segment; body drag moves both anchors in time and price; either endpoint can be edited independently.
- **Ray:** two anchors define origin and direction; rendering extends only from origin through the second anchor toward the right; body and endpoints remain editable.
- **Rectangle:** two opposite corners define time/price bounds; filled area and border are selectable; body drag moves the whole rectangle; corner handles resize it without corrupting anchor order.
- **Fibonacci Retracement:** two directional anchors; visible standard levels at least 0, 0.236, 0.382, 0.5, 0.618, 0.786, and 1; each level shows ratio and price; reversing/editing direction updates levels deterministically; body and both anchors are editable.
- **Text/Note:** one price/time anchor plus explicit text entry before commit or immediately after placement; empty/cancelled text creates no orphan; selected text can be edited and moved; persist text content as Sumi domain data with the documented length limit.

Creation tools must support preview without writing persistence/history per pointer move. Multi-anchor creation must make one history command only after the final valid anchor/text commit. Escape, Cursor, pointer cancellation, route unmount, and provider destroy must remove incomplete previews and restore chart scrolling/pointer capture without revision changes.

## Domain, schema, and migration

Use a Sumi-owned discriminated drawing union independent of provider-native state. Align the TypeScript implementation and fixtures with `docs/decision-packs/sumi-drawing-document-v1.schema.json`:

- stable UUID, tool, pane ID, order, visible/locked state;
- one or two time/price anchors according to tool;
- explicit style including relevant line/fill/text properties;
- Fibonacci levels and direction as first-class data;
- Text content as first-class data;
- document schema version, revision, session ID, and symbol.

Do not silently change the meaning of schema version 1. If the existing design schema and the production horizontal-only validator cannot be reconciled compatibly, stop and request a Reviewer schema decision rather than inventing an undocumented version.

Implement strict validation and fixtures for every tool, malformed/unknown fields, duplicate IDs, order, invalid anchors, invalid style, invalid Fib levels/direction, excessive text, session/symbol mismatch, and future schema versions.

Implement non-destructive migration for both existing sources:

1. current identity-keyed/local Horizontal `SumiDrawingDocumentV1` records;
2. legacy backend `state_data` arrays handled by `SumiDrawingAdapter` (`cursor`, `trendline`, `horizontal`, `fibonacci`) where the record is semantically valid.

Migration requirements:

- never mix sessions or symbols;
- preserve original IDs, anchors, color, and supported semantics where possible;
- never promote legacy cursor artifacts as real drawings;
- preserve the original raw legacy payload under a documented backup/rollback key before first canonical write;
- reject or quarantine ambiguous/malformed records rather than fabricate geometry;
- migration is idempotent and does not duplicate drawings across reload;
- record explicit consequences for legacy Fibonacci data that lacks levels/direction or other required semantics.

## Persistence authority and failure semantics

Use the existing local-first Sumi repository and the existing opaque replay drawing `state_data` endpoint; do not add a database migration merely to store provider-native data. The ExecPlan must define one unambiguous canonical/mirror/conflict policy before implementation.

Required properties:

- canonical serialized state is Sumi document JSON, never provider JSON;
- committed create/change/delete/clear/undo/redo is serialized per session/symbol;
- backend persistence uses the isolated local Sumi backend and closes the retained `drawings.persist-after-create` failure;
- local identity storage remains safe for offline/local recovery;
- stale revisions or divergent valid copies cannot silently overwrite newer work;
- persistence failure is visible and does not falsely advance history or report a committed operation;
- migration and writes are fully covered by temporary-DB integration tests; automated work must not mutate `backend/sumi.db`.

If the existing opaque endpoint is insufficient to provide the recorded conflict policy without a backend contract change, stop with evidence and a narrow decision request. Do not hide the limitation by treating reload-from-localStorage as proof of backend persistence.

## Provider and geometry architecture

Keep all chart-native calls, pixel/time/price conversion, hit testing, drawing primitives, pointer capture, and provider lifecycle behind `DrawingProvider`/`SumiPrimitiveDrawingProvider`. React UI, domain history, and persistence must not depend on Lightweight Charts native objects.

Required provider properties:

- use official time-scale and series coordinate APIs; do not infer candle spacing from DOM or private internals;
- interact only inside the official price-pane element; indicator/Volume pane clicks must not create or edit drawings;
- render all tools from one ordered Sumi document without one listener set per drawing;
- deterministic z-order and topmost hit selection;
- body and anchor hit regions are distinct and remain usable at both required viewport sizes;
- selected drawings show unambiguous handles/bounds; unselected drawings do not display editing handles;
- body moves preserve each tool's geometry; anchor edits change only the intended anchor;
- pan/zoom/resize/replay updates reproject from domain anchors and never write geometry merely because coordinates changed;
- pointer move previews are ephemeral and coalesced; one completed gesture emits one semantic commit;
- `destroy()` is idempotent and removes all DOM/chart listeners, primitives, previews, capture, selection, and subscriptions.

Extend `DrawingInteractionSnapshot` with actual tool/anchor/body/coordinate state sufficient for semantic UAT. Do not add hidden hard-coded acceptance surrogates.

## Magnet/snapping contract

Implement a visible, persisted or explicitly workspace-scoped magnet control with at least Off and OHLC snapping modes. Record the exact rule before coding:

- snap only to **visible replay candles** returned through the current index; never future candles;
- use a documented pixel threshold and deterministic nearest-candidate/tie-break policy;
- apply consistently during creation and anchor/body editing where appropriate;
- make active magnet mode visually obvious and keyboard/mouse accessible;
- Off must preserve unsnapped coordinates;
- Escape/pointer cancellation must rollback the exact pre-gesture anchors regardless of magnet mode.

Do not claim magnet support from a function that returns unchanged anchors. Focused tests and browser UAT must prove both a snapped and an unsnapped result.

## Commands, selection, and toolbar UX

Generalize the existing price-only selection toolbar without losing the accepted Horizontal behavior:

- required tools have readable labels/tooltips and obvious active state;
- selected tool/type/ID and relevant fields are understandable;
- edit fields are tool-aware (anchors, style, Fibonacci direction/levels, text) and validate before commit;
- UI and keyboard Delete/Backspace delete the exact selected drawing;
- Escape cancels active creation/drag/edit before it deselects;
- Clear All requires confirmation and remains undoable;
- undo/redo covers create, body move, anchor edit, settings/text edit, delete, and clear;
- a drag or multi-step creation produces one history entry, not one entry per preview event;
- undo/redo restores exact semantic document and selection where recorded;
- revision conflict/persistence failure leaves history unchanged and shows honest feedback.

## Focused test requirements

Add focused domain/repository/history/provider/geometry/UI tests for at least:

- valid and invalid schema fixtures for every tool;
- legacy local/backend migration, backup, idempotence, symbol/session isolation, and malformed quarantine;
- create/cancel/pointercancel/destroy lifecycle for every tool;
- exact body move and each editable anchor/corner;
- ray direction/extension; rectangle normalization; Fibonacci levels, direction reversal, labels/prices; text create/edit/cancel;
- topmost hit testing, deselection, handle vs body hit, lock/visibility behavior;
- magnet Off/OHLC, threshold edge, deterministic tie, and visible-candle-only no-future behavior;
- one semantic command per completed gesture and exact undo/redo for every command kind;
- CAS/persistence conflict and backend/local failure semantics without history corruption;
- pan/zoom/resize/replay reprojection without domain mutation;
- duplicate time anchors, null coordinate conversion, rapid tool switching, pointer capture loss, and idempotent destroy;
- ten remounts with one subsequent input producing exactly one action/revision.

## Product UAT

Extend `scripts/product-uat.mjs` additively. Do not delete, weaken, rename away, or relabel any existing assertion. All accepted Batch 1/2 checks remain blocking. The seven current drawing failures must turn green only through real exercised behavior.

For each required tool, browser UAT must prove:

- toolbar presence, label/tooltip, active highlight, and Escape/Cursor cancellation;
- creation with exact stable ID/tool/anchors/geometry and no incomplete orphan;
- visible selection bounds/handles and empty-space deselection;
- body move and independent anchor/corner edit as applicable;
- UI deletion and keyboard deletion of the exact ID;
- exact undo and redo of create, move/edit, and delete;
- persistence to the existing backend endpoint plus local identity state, reload equality, and no duplicate promotion;
- alignment and selectability after pan, zoom, replay next/previous, pane layout changes, 1440×1000, and 1280×800.

Additionally prove:

- Fibonacci standard levels, visible ratio/price labels, direction reversal, body move, endpoint edit, reload equality;
- Text nonempty commit, empty/Escape cancellation, text edit, move, delete, undo/redo, reload equality;
- magnet visibly switches Off/OHLC, performs a known OHLC snap inside threshold, refuses a candidate outside threshold, and never uses a future candle;
- non-price-pane isolation for all creation tools;
- clear confirmation and exact undo restoration of all tool types;
- ten mount/unmount cycles followed by one action creates one drawing and one revision only;
- zero page/console/runtime/provider errors, duplicate times, recursion, stale listeners, or orphan primitives.

Capture and manually review at minimum:

- 1440×1000 with all required toolbar tools and representative selected drawings;
- selected/moved/edited handles and bounds;
- Fibonacci labels/direction;
- Text edit state;
- pan/zoom/replay state;
- post-reload state;
- 1280×800 compact state with no clipped core drawing controls.

Machine evidence must contain semantic domain/provider snapshots, not only “page is not blank” or hidden declared strings.

## Regression and verification gates

Record production DB SHA-256 before work. Run at minimum:

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

Use an isolated temporary database for all backend/UAT writes. Preserve Batch 2 indicator layout/chrome/request checks, including zero active errors and the fixed responsive policy. Retain machine-readable output, screenshots, migration fixtures, and exact backend/local persistence evidence.

## Stop and escalate instead of guessing

Stop at Reviewer gate with evidence and a narrow decision request if:

- official primitives cannot provide reliable multi-anchor time/price interaction, hit testing, or cleanup;
- schema v1 compatibility requires changing already persisted semantics;
- safe migration/conflict handling requires a backend contract or database migration not justified by the existing opaque endpoint;
- Fibonacci or Text cannot round-trip without provider-specific state;
- magnet would require future candles or frontend access to undisclosed replay data;
- the implementation appears to require a community dependency, Lightweight Charts fork/private API, chart-library replacement, weakened acceptance criterion, or removed UAT assertion.

Do not begin Batch 4, trade-layout redesign, journal/checklist work, or unrelated cleanup.

## Final DEV handoff

Update the ExecPlan with decisions, migration policy, progress, exact verification output, evidence paths, remaining failures, deviations, and self-review. End with:

`BATCH 3 DEV COMPLETE — STOPPED AT REVIEWER GATE`

Report:

- D-01–D-11 result individually;
- exact interaction contract and implemented semantics per tool;
- schema/migration/persistence result and backup paths;
- provider/listener/primitive ownership and cleanup result;
- magnet rule and snapped/unsnapped/no-future evidence;
- focused/full frontend and backend test counts;
- final UAT pass/fail/`blockingFailed`, every remaining failure ID, and request/runtime/provider error counts;
- 1440×1000 and 1280×800 artifact paths plus key tool-specific artifacts;
- production DB before/after SHA-256;
- deviations and known limitations.

Do not claim Sumi is product-complete or professionally ready. Do not start Batch 4. Stop for Reviewer inspection.
