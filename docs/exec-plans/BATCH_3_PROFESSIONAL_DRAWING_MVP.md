# Batch 3 — Professional Drawing MVP

## Outcome

Deliver D-01 through D-11 for Cursor/Select, Horizontal Line, Trendline, Ray, Rectangle, Fibonacci Retracement, and Text/Note using the approved Sumi-owned domain and `SumiPrimitiveDrawingProvider` over official Lightweight Charts v5 primitive/time/price APIs. Every committed drawing supports selection, editing, deletion, undo/redo, canonical persistence, reload, viewport/replay stability, magnet behavior, and deterministic cleanup.

## Second Reviewer closure — B3-R07–B3-R10

### Authorization, provenance, and preserved dirty inventory

- Authorized only by `docs/dev-prompts/BATCH_3_SECOND_REVIEW_CLOSURE_PROMPT.md` to close B3-R07 through B3-R10 from the independent hardening re-inspection in `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`. Batch 3 remains unapproved; Batch 4 and Batch 5 remain unauthorized.
- Before second-closure edits the checkout is `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; HEAD has no exact tag. The protected `v2.0.0-rc2` tag is not moved, rewritten, or retagged.
- Production DB SHA-256 before second closure: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Preserved tracked dirty inventory: `.gitignore`, `docs/AGENTS.md`, `docs/INDEX.md`, `frontend/package.json`, `frontend/src/api/indicatorsApi.ts`, `frontend/src/components/chart/{CandleChart.tsx,DrawingToolbar.tsx,IndicatorRenderRegistry.ts,PaneManager.ts,SeriesManager.ts,workspaceTypes.ts}`, `frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts`, `frontend/src/hooks/useWebSocket.ts`, `frontend/src/pages/ReplayPage.tsx`, and `frontend/vite.config.ts`.
- Preserved untracked inventory: root `AGENTS.md`/`PLANS.md`; canonical V3 docs; all `docs/decision-packs`, `docs/dev-prompts`, `docs/exec-plans`, `docs/review-artifacts`, `docs/reviewer-prompts`, and `docs/reviews`; new Replay/chart/indicator/drawing modules and tests under `frontend/src/components` and `frontend/src/features`; product/provider UAT scripts; and `spikes/`. No existing user/Reviewer/DEV file or evidence is discarded, staged, reorganized, or overwritten.
- Immutable browser baseline: `test-results/product-uat/2026-07-18T03-50-26-303Z/results.json`, exactly 216 passing IDs with their current names, pass values, and blocking semantics. The final comparison must report missing, duplicate, renamed, changed-pass, and changed-blocking IDs explicitly.

### Exact scope and affected modules

| Finding | Modules | Bounded outcome |
| --- | --- | --- |
| B3-R07 | `docs/decision-packs/sumi-drawing-document-v1.schema.json`, `drawingDomain.ts`, a shared fixture corpus and `drawingDomain.test.ts`, plus UAT corpus evidence | Align all Draft 2020-12-expressible structural rules; publish structural schema plus named supplemental Sumi semantic invariants; validate one positive/negative corpus with an actual Draft 2020-12 validator and runtime semantics separately. |
| B3-R08 | `DrawingRepository.ts`, `useDrawingWorkspaceController.ts`, controller/repository focused tests and controlled UAT API routing | Reconcile every indeterminate PUT/echo outcome with a serialized GET, commit or rollback exactly once when remote is intended/prior, and block with preserved prior/intended recovery evidence for divergent/unreadable remote; use the same decision table for migration and history commands. |
| B3-R09 | `SumiPrimitiveDrawingProvider.ts` and focused provider tests | Convert multi-anchor body drag atomically: project original anchors, calculate every logical-time/price translation, and reject the entire gesture if any official conversion is null, invalid, nonpositive, or not a real session date. |
| B3-R10 | `scripts/product-uat.mjs` and only the narrow browser-visible provider/controller seams needed for deterministic failure routing | Strengthen endpoint/corner/body assertions to exact field and equal-delta semantics; prove partial conversion rejection, reconciliation outcomes, and the shared schema/runtime corpus in machine evidence while retaining all 216 baseline IDs. |

The dormant legacy `DrawingToolRegistry` remains outside Replay mutation ownership. If its default-empty instantiation is still present, remove it only when the current facade proves no compatibility consumer; otherwise retain and document/test it as strictly empty/read-only. It must not become a second mutable authority.

### Canonical v1 strategy and supplemental semantic invariants

- The published canonical v1 contract is the Draft 2020-12 structural schema plus explicitly named Sumi semantic invariants that JSON Schema cannot express safely here: Ray anchor 1 is strictly later/right of anchor 0; drawing IDs are unique; `order` values are unique and contiguous from zero; and document/repository session/symbol identity must match the requested workspace.
- All expressible rules are aligned in schema/runtime, including nonempty optional colors, exact tool/geometry/anchor shapes, and real Gregorian calendar dates. Date semantics use a bounded Draft 2020-12-compatible structural expression plus the shared corpus; runtime remains authoritative for supplemental cross-item/document relationships only.
- One shared fixture corpus covers valid Horizontal/all-tools/Text/Fib cases and malformed, future-version, unknown-field/tool/geometry, anchor-count, pane, Fib ratios/colors, Text blank/length, impossible-date, Ray left/equal, duplicate-ID/order, and identity cases. Tests run each case through an actual Draft 2020-12 validator and `parseDrawingDocument`, then assert `expected.structural` and `expected.semantic` separately rather than claiming validator equivalence from constants.
- Existing valid Horizontal v1 serialization and meaning remain unchanged. Discovery of any supported nonstandard canonical v1 payload is a stop condition requiring exact payload evidence and Reviewer versioning direction.
- No production runtime dependency is added. If the current dependency graph cannot provide an explicit, lockfile-backed dev-only Draft 2020-12 validator with acceptable license posture, stop before substituting an incomplete validator.

### Indeterminate-write reconciliation state machine

Within the one session/symbol queue, every migration, ordinary commit, undo, and redo follows the same state machine:

1. GET and verify the expected prior remote mirror.
2. Prepare intended revision and local CAS candidate without publishing UI/history success.
3. PUT the canonical intended document.
4. Exact normal echo means `intended`: accept local/UI/history once.
5. A dispatched request failure or mismatched echo triggers one serialized reconciliation GET before any outcome is published.
6. Reconciled remote equal to intended means `intended`: accept once; equal to prior means `prior`: restore local/UI/history exactly.
7. A third/divergent value or unreadable reconciliation means `blocked`: keep the last committed document active, preserve exact prior and intended canonical recovery copies plus observed/error evidence, expose an explicit indeterminate/conflict status, and reject later writes until reload/reconciliation.

The endpoint has no atomic database CAS and this closure makes no cross-client atomicity claim. Tests must model PUT committing before throwing and PUT mutating remote while returning a mismatched echo; a mock that throws before mutation is insufficient proof.

### Atomic body-drag conversion policy

- Body movement projects the original drawing, computes one pixel translation, converts every translated anchor through official `coordinateToTime` and `coordinateToPrice`, validates every resulting date/price, and only then publishes a preview.
- Any one-anchor time or price conversion failure rejects the whole preview and marks the transaction non-committable. No anchor may fall back to its original while another moves.
- Successful two-anchor movement must preserve identical logical candle-index deltas and identical price deltas for Trendline, Ray, Rectangle, and Fibonacci, including Ray direction, Rectangle geometry, and Fib anchor/direction semantics.
- Rejected movement leaves the provider/controller document, revision, command history, local/backend state, selection, pointer capture, scrolling, and dirty transaction state exactly unchanged. Pointerup cannot commit a previously rejected preview.

### UAT retention and additive/strengthened IDs

- Retain all 216 baseline IDs exactly. Strengthen their boolean conditions where B3-R10 requires exact semantics without renaming them.
- Add blocking IDs under `batch3.second-closure.*` for: shared schema/runtime corpus structural and supplemental results; ordinary commit-then-error intended/prior/divergent/unavailable reconciliation; mismatched-echo reconciliation; undo/redo acceptance where relevant; migration intended/prior/divergent/unavailable outcomes; provider partial-time and partial-price rejection with exact clean state; and exact body logical-index/price deltas.
- Real-pointer assertions cover Trendline/Ray/Fib endpoints 0/1, Text anchor 0, all Rectangle corners with nonzero x and y, exact changed/unchanged time/price source fields, strictly rightward Ray, and equal body deltas. Waits remain observational and are followed by exact state/persistence/render assertions.
- Fresh retained screenshots remain mandatory at 1440x1000 for all tools/handles, four Rectangle x+y corners, multiline Text bounds, inspector, pan/zoom/replay/reload and at 1280x800 for open inspector, usable chart, and non-overlapping core controls. Runtime/page/console/provider/indicator/request failures remain blocking.

### Rollback, compatibility, stop conditions, and verification

- Rollback is source-only: restore the prior controller/provider/schema/UAT implementation while retaining recovery evidence and existing valid v1 documents. No backend schema/database rollback is required. Existing Horizontal v1 records are unchanged; no provider-native data is introduced.
- Stop rather than guess if work requires schema v2, reinterpretation of supported nonstandard v1, backend contract/database migration, cross-client atomicity, a dependency/provider/fork/private API/chart change, weakening/removing/renaming an assertion, broad Replay/trade/journal/Indicator redesign, or Batch 4/5 work.
- Exact commands:

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

- All automated/UAT backend writes use isolated temporary databases. Retain result directories, screenshots, backend/frontend logs, and machine-readable output; finish with a diff/acceptance self-review and stop at the Reviewer gate.

## Reviewer hardening — B3-R01 through B3-R06

### Authorization and baseline

- Authorized only by `docs/dev-prompts/BATCH_3_REVIEW_HARDENING_PROMPT.md` to close B3-R01–B3-R06 from `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`. Batch 3 remains unapproved and Batch 4 remains unauthorized.
- Continue on the existing dirty `master` checkout at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; preserve every user/Reviewer/Batch 1/2/3 file and do not branch, worktree, commit, push, merge, reset, clean, checkout, retag, discard, or overwrite.
- Reviewer baseline is `test-results/product-uat/2026-07-18T01-37-13-462Z/results.json`: 174 passed / 0 failed / 0 blocking failed. All 174 IDs, names, and blocking status are immutable inputs to this hardening; new proof is additive.
- Production DB SHA-256 before hardening: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

### Finding-to-module closure plan

| Finding | Exact modules | Closure and proof |
| --- | --- | --- |
| B3-R01 | `docs/decision-packs/sumi-drawing-document-v1.schema.json`, `drawingDomain.ts`, `DrawingRepository.ts`, fixtures and `drawingDomain.test.ts` | Tighten pre-approval schema v1 to `paneId: price`, exact ordered standard Fib levels, trimmed nonempty Text up to 2000, and exact tool/geometry/anchor mapping. Add a dependency-free JSON-schema/runtime drift contract test and byte-semantic Horizontal round-trip proof. |
| B3-R02 | `DrawingRepository.ts`, `useDrawingWorkspaceController.ts`, controller/repository tests | Make hydration/promotion/echo and later commands share one per-identity serialized authority; publish `ready` only after verified echo; blocked quarantine never becomes editable; derive every queued mutation from latest committed refs; prove two immediate edits and exact UI/local/backend/history rollback on conflict/failure. |
| B3-R03 | `SumiPrimitiveDrawingProvider.ts`, `CandleChart.tsx`, Replay tool wiring and provider tests | Make every tool switch, Escape/Cursor, `pointercancel`, lost capture, unmount and idempotent destroy use exact rollback/cleanup. Reject null/invalid official time/price/pane conversions with no current-date fallback and no dirty capture/scroll/listeners. |
| B3-R04 | `drawingGeometry.ts`, `SumiPrimitiveDrawingProvider.ts`, `drawingDomain.ts`, geometry/provider tests | Enforce strictly rightward Ray creation and pointer/inspector edit; expose all four Rectangle corners with deterministic two-anchor field mapping; implement bounded explicit-newline Text layout shared by render/hit test; preserve one primitive/listener set and semantic body deltas. |
| B3-R05 | `DrawingToolbar.tsx`, new narrow Sumi drawing inspector component if useful, `ReplayWorkspace.tsx`, controller/domain callbacks | Keep the icon rail compact and move properties to a responsive labelled inspector outside it. Expose stable ID, anchors, line/fill/text style, visibility, lock, Fib direction and Text. Each accepted change is one serialized command; invalid/cancelled drafts do nothing; focused keyboard isolation prevents chart Delete/Escape/navigation while editing. |
| B3-R06 | drawing focused tests and `scripts/product-uat.mjs` | Retain the current 174 checks unchanged and add real canvas gestures, exact before/after anchors/revision evidence, schema/transaction/lifecycle/geometry/inspector checks and required wide/compact screenshots. Form edits are recorded separately from canvas handle gestures. |

### Compatibility and schema decision

- Batch 3 was not approved before this correction. Approved persisted v1 product support is Horizontal-only; no approved non-Horizontal canonical v1 record exists. Existing valid Horizontal documents retain the same fields, meaning, serialization and round-trip behavior.
- Correct schema v1 to the already recorded product semantics rather than create v2: price pane only; exact ordered ratios `0, .236, .382, .5, .618, .786, 1` with `visible` and optional `color`; Text is nonempty after trimming and at most `TEXT_MAX_LENGTH = 2000`; geometry kind and anchor count exactly match the tool.
- Development-only malformed/nonstandard payloads are quarantined and visibly blocked, never reinterpreted or partially promoted. If evidence of a previously supported canonical nonstandard Fib/pane record appears, stop before modifying it and return its payload shape to Reviewer.

### Interaction, transaction and rollback policy

- Provider tool changes first rollback any drag to its exact before drawing and selection, or discard a two-anchor preview, then release capture, restore scrolling and activate the requested tool. Escape/Cursor/cancel/lost capture/unmount/destroy share the same idempotent cleanup; rejected coordinates produce no event/document/history/revision/local/backend mutation.
- Ray requires anchor 1 to project strictly later/right of anchor 0. Invalid create remains an uncommitted preview; invalid pointer/inspector edit retains the prior valid Ray without history or persistence.
- Rectangle exposes four projected handles while keeping two canonical anchors: TL maps `(a.time, maxPrice)`, TR maps `(b.time, maxPrice)`, BR maps `(b.time, minPrice)`, BL maps `(a.time, minPrice)` according to projected left/right/top/bottom fields; dragging a mixed corner changes exactly its time-source and price-source fields without reordering the domain anchors.
- Text uses a bounded multiline canvas layout with explicit newline preservation, deterministic wrapping, a maximum rendered width, and identical measured bounds for draw/hit/select. Canonical content preserves accepted newlines after outer trim.
- One per-session/symbol queue owns hydration, migration and all semantic commands. Work is derived from latest committed refs at execution time. Failed migration retains backup/local recovery but remains `error`; quarantined/ambiguous remote remains `conflict`; any conflict/failure restores React/local/backend-mirror/history equality and pauses writes. The opaque backend still provides no cross-client database CAS, and no such claim is made.
- The inspector uses drafts and commits only valid changed values on explicit Apply. Escape/Cancel restores the draft only; inputs stop chart keyboard commands. Visibility/lock/style/anchor/Text/Fib changes each create exactly one history command and one durable revision.

### Additive UAT IDs

- Add `batch3.hardening.schema-runtime-contract`, `batch3.hardening.horizontal-v1-compatibility`, migration readiness/failure/quarantine/rapid-edit/conflict equality IDs, tool-switch/null-coordinate/capture-loss/destroy IDs, Ray invalid-direction IDs, per-handle and per-body canvas gesture IDs, multiline Text bounds ID, inspector property/cancel/invalid/history/reload/containment/keyboard IDs, and post-ten-remount single-response ID.
- Every new check is blocking under the existing `batch3.` classifier. Do not remove, rename, weaken, skip, or make nonblocking any current ID.

### Hardening verification and stop conditions

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

- Use only isolated temporary UAT/backend state and retain machine results, screenshots and logs. Compare the 174-ID Reviewer baseline against the final run for missing/renamed/weakened checks.
- Stop with evidence if closure requires schema v2/reinterpreting supported nonstandard v1, backend contract/database migration/cross-client CAS, a dependency/fork/private API/chart change, assertion weakening, broad Replay/trade/journal redesign, or Batch 4 work.

## Context and authorization

- Authorized only by `docs/dev-prompts/BATCH_3_PROFESSIONAL_DRAWING_MVP_PROMPT.md` after Batch 2 final closure in `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`.
- Batch 1 Horizontal and Batch 2 Indicator Manager are Reviewer-approved baselines and remain blocking regressions.
- Provider decision remains `SumiPrimitiveDrawingProvider`; Deepentropy and Difurious are rejected and no dependency/chart-library change is allowed.
- Batch 4 and unrelated replay/trading/journal redesign are out of scope.

## Checkout provenance and baseline

- Work directly on current `master` checkout at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; no branch/worktree/switch/checkout/commit/push/merge/reset/clean/retag/discard/overwrite.
- Preserve the existing dirty Reviewer/user/governance/harness/evidence plus approved Batch 1/2 implementation. `v2.0.0-rc2` remains at `66acf862a10c850185d4993237191fba879da7ca` and is untouched.
- Production DB SHA-256 before Batch 3: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Baseline `./scripts/verify-v2.sh`: backend 75 passed / 1 skipped; frontend 16 files / 57 tests; lint/build pass.
- Baseline isolated UAT: `test-results/product-uat/2026-07-18T00-53-20-030Z/results.json` — 79 pass / 7 retained failures / `blockingFailed: 0`; zero runtime errors and zero indicator request failures.
- The seven authorized gaps are exactly `drawings.tool-trendline`, `drawings.tool-ray`, `drawings.tool-rectangle`, `drawings.tool-fibonacci`, `drawings.tool-text`, `drawings.selection-contract`, and `drawings.persist-after-create`.

## In scope

- Full drawing union/schema validation, fixtures, canonical serialization, local/backend legacy migration, backup, quarantine, and local/backend persistence transactions.
- One provider primitive and listener set for all required price-pane drawings; creation previews, hit testing, selection, body/anchor editing, magnet, lifecycle cleanup, and semantic snapshots.
- Tool-complete toolbar/selection editor, text entry, visible magnet control, keyboard contract, exact delete/clear/history semantics.
- Additive focused tests and browser UAT for D-01–D-11 while preserving all accepted Batch 1/2 assertions.

## Out of scope

- New community drawing/provider/chart dependencies, Lightweight Charts internals/private APIs, backend schema/database migration, production record migration, telemetry, Batch 4, trade layout, journal/checklist work, broad cleanup, or indicator architecture changes.

## Product and data invariants

- Only visible replay candles supplied through `current_index` are exposed to the drawing provider and magnet; no future candle is sent or queried.
- Domain documents contain Sumi semantic JSON only—never native primitives, coordinates, canvas objects, or provider JSON.
- Drawing interaction is price-pane-only. Indicator/Volume panes cannot create, select, or edit drawings.
- Pan/zoom/resize/replay changes only reprojection; they never mutate anchors, revision, persistence, or history.
- Preview state is ephemeral. One completed create/edit gesture produces at most one persistence transaction and one history command.
- Automated backend/UAT writes use a temporary DB; `backend/sumi.db` remains byte-for-byte unchanged.

## Exact interaction contract

- **Cursor/Select:** click the topmost visible unlocked body/handle by document order; a handle wins over a body at equal distance. Empty price-pane click deselects. Select never creates. Locked drawings render but are not selected/edited.
- **Escape precedence:** cancel current text entry, creation preview, or drag and restore its exact before-state; then return to Select without a revision. With no active transaction it returns to Select and preserves current selection; a subsequent empty click deselects.
- **Cursor button/tool switch:** switching to Select cancels incomplete creation/drag/text exactly. Switching between creation tools cancels the previous preview before activating the new tool.
- **Horizontal Line:** one price-pane click commits one horizontal at the clicked/snapped price. Body or right-side handle vertical drag changes only price.
- **Trendline:** first click starts preview, pointer movement previews, second click commits a finite two-anchor segment. Body drag shifts both anchors by the same logical-time index delta and price delta; endpoint handles edit only their anchor.
- **Ray:** two anchors commit; render begins at anchor 0, passes through anchor 1, and extends only rightward to the price-pane edge. Body/endpoint behavior matches Trendline while preserving direction.
- **Rectangle:** two opposite corners commit. Render normalizes pixel bounds without reordering domain anchors. Fill/border select the body; body drag shifts both anchors; each corner handle edits only its corresponding directional anchor.
- **Fibonacci Retracement:** two directional anchors commit. Levels are exactly `0, .236, .382, .5, .618, .786, 1`; each renders ratio and interpolated price. Canonical schema direction is `start-to-end` while the anchor order is retained; an explicit direction-reversal edit swaps to `end-to-start`, and level prices interpolate deterministically from the selected direction. Body shifts both anchors; either endpoint edits independently.
- **Text/Note:** one click opens explicit text entry anchored at that point. Trimmed text length 1–2000 commits once; empty text, Cancel, Escape, pointer cancellation, tool switch, unmount, or destroy creates no drawing. Selected text can be edited with validation and body/anchor drag moves its single anchor.
- **Delete/clear/history:** UI Delete and keyboard Delete/Backspace remove the exact selected ID. Confirmed Clear All is one undoable command. Undo/redo covers create, body move, anchor edit, settings/text edit, delete, and clear and restores exact semantic state.

## Magnet contract

- Visible workspace control has `Off` and `OHLC`; the mode is workspace-scoped per session/symbol in local storage and never embedded in provider/native JSON.
- OHLC considers only candles currently supplied to the chart, hence only replay-visible candles. Candidate x is the nearest visible logical candle coordinate within **10 CSS pixels**; candidate y is that candle's O/H/L/C coordinate within **10 CSS pixels**.
- Minimize squared pixel distance. Deterministic ties use candle logical order, then price field priority `open`, `high`, `low`, `close`. A candidate must satisfy both thresholds; otherwise the raw time/price is retained.
- Creation and individual anchor edits snap. Body moves preserve geometry/deltas and do not independently deform anchors by snapping. `Off` always retains raw converted anchors. Escape/pointercancel restores the exact pre-gesture anchors in either mode.

## Domain and schema policy

- Keep schema version 1 and align TypeScript exactly to `docs/decision-packs/sumi-drawing-document-v1.schema.json`; the existing Horizontal records are a valid subset, so no semantic version change is required.
- Discriminated union: Horizontal (one anchor/horizontal geometry); Trendline and Ray (two anchors/line geometry with extension); Rectangle (two anchors/rectangle geometry); Fibonacci (two anchors/fibonacci geometry with standard levels/direction); Text (one anchor/text geometry with content and text style).
- Strict validation rejects unknown document/drawing/anchor/style/geometry fields, unknown tools, future versions, non-UUID/duplicate IDs, non-contiguous order, invalid pane/visibility/lock, invalid dates/prices, incorrect anchor counts, invalid line/fill/text style, nonstandard/duplicate Fib levels/direction, text over 2000, and mismatched session/symbol at repository boundaries.

## Migration, backup, and quarantine policy

- Canonical identity key remains `sumi:drawing-document:v1:<sessionId>:<encodedSymbol>`; valid current Horizontal-only documents load unchanged.
- Backend canonical document is accepted only for the requested session/symbol and after strict validation. Valid legacy arrays promote only `horizontal`, two-point `trendline`, and two-point `fibonacci`; `cursor` is always ignored. Legacy `ray`, rectangle, and text are not fabricated because the old adapter did not define them.
- Legacy IDs/anchors/color are preserved when valid; generated style defaults and geometry are documented Sumi semantics. Legacy Fibonacci becomes the standard seven levels and derives direction from anchor prices; this is the explicit consequence of missing old metadata.
- Before the first canonical write, preserve raw backend legacy payload at local rollback key `sumi:drawing-legacy-backup:v1:<sessionId>:<encodedSymbol>`; never overwrite an existing backup.
- Invalid or ambiguous payloads are copied to `sumi:drawing-quarantine:v1:<sessionId>:<encodedSymbol>` with source/reason/raw value and are never partially promoted.
- Promotion writes once and is idempotent: canonical IDs are de-duplicated, the same backend legacy payload cannot append again on reload, and cross-session/symbol data is rejected.

## Persistence authority, mirror, and conflict policy

- **Canonical working authority:** the valid identity-keyed local Sumi document, because Sumi is local-first and must remain usable offline. **Durable mirror/resume source:** the existing backend `state_data`, serialized as the same canonical Sumi document JSON.
- Hydration compares valid local and backend documents for the exact session/symbol: only byte-equivalent canonical copies reconcile automatically. Any divergent canonical pair—regardless of revision—or simultaneous nonempty local canonical/backend legacy state becomes a visible conflict and neither copy is overwritten. Legacy arrays are migration inputs only when canonical state is absent, not authorities after canonical promotion.
- Every committed create/change/delete/clear/undo/redo performs a serialized transaction: preflight GET backend; require its canonical revision/content to match the controller's expected mirror (or documented empty/legacy migration state); prepare local CAS; PUT the next canonical document; require echoed canonical equality; then commit history/UI. Any failure restores the prior local value, leaves UI/history/revision unchanged, and shows an honest error.
- The opaque endpoint has no database CAS. Batch 3 therefore permits only one serialized client writer per session/symbol and detects stale/divergent remote state before each write plus verifies the response afterward. Any observed concurrent race/divergence stops further writes and requires reload/reconciliation; it is never silently overwritten. No backend contract change is introduced.
- Offline failure retains the last committed local document as recovery but marks backend mirror unavailable; a new semantic commit is not reported successful until backend persistence succeeds. Backend/local transaction and migration integration tests run against temporary storage/DB fixtures.

## Provider and component architecture

```text
ReplayWorkspace (composition)
  -> DrawingToolbar (tools, magnet, selection editor, text dialog)
  -> useDrawingWorkspaceController (domain commands, serialized persistence, history)
  -> CandleChart facade
       -> SumiPrimitiveDrawingProvider
            one SumiDrawingDocumentPrimitive
            one chart/pointer listener set
            official timeScale/series coordinate conversion
            geometry projection + hit testing + preview + snapshot
  -> DrawingRepository (canonical/local/backend migration policy)
```

- Provider owns all chart-native calls, price-pane bounds, coordinate conversion, hit testing, primitive drawing, pointer capture, scroll suppression/restoration, preview, and cleanup.
- Controller owns semantic mutations, persistence, revision/history, selection, magnet workspace state, and visible persistence/conflict status.
- Toolbar understands domain fields only. React and repository never receive native chart objects or pixels.

## Affected modules

- Refactor `frontend/src/features/drawings/drawingDomain.ts`, `DrawingRepository.ts`, `DrawingCommandHistory.ts`, `DrawingProvider.ts`, `SumiPrimitiveDrawingProvider.ts`, `useDrawingWorkspaceController.ts` and their tests/fixtures.
- Add focused pure geometry/magnet helpers and tests under `frontend/src/features/drawings/`.
- Generalize `frontend/src/components/chart/DrawingToolbar.tsx`, `CandleChart.tsx`, and `workspaceTypes.ts` narrowly.
- Update drawing integration in `frontend/src/components/replay/ReplayWorkspaceController.tsx`, `ReplayWorkspace.tsx`, and `frontend/src/api/replayApi.ts`; retain legacy `SumiDrawingAdapter` only as migration parser/compatibility evidence.
- Extend `scripts/product-uat.mjs` additively; no existing assertion ID is removed, renamed, weakened, or made nonblocking.
- Add temporary-DB API integration coverage only if needed without changing the endpoint/database schema.

## D-01–D-11 mapping

| ID | Implementation | Verification evidence |
| --- | --- | --- |
| D-01 | Discoverable seven-tool toolbar, active state, tooltips/help, Select contract | Toolbar semantics and creation/cancel UAT per tool |
| D-02 | Complete Horizontal lifecycle retained | Existing Batch 1 suite plus backend persistence |
| D-03 | Trendline finite segment/body/endpoints | Geometry/provider focused tests and browser gestures |
| D-04 | Ray directional right extension/body/endpoints | Semantic snapshot and selected ray screenshot |
| D-05 | Rectangle fill/border/body/corners | Normalization tests and browser corner edits |
| D-06 | Fib directional levels/labels/body/endpoints | Level/price/direction snapshot and screenshots |
| D-07 | Text placement/entry/edit/move/cancel | Domain/UI/provider tests and visible edit screenshot |
| D-08 | Topmost selection, handles/bounds, exact delete/clear | Hit-order tests and UI/keyboard UAT |
| D-09 | One-command create/edit/delete/clear with exact undo/redo | History/controller tests and revision evidence |
| D-10 | Canonical local/backend persistence, migration, reload | Fixtures, temp integration tests, endpoint/local evidence |
| D-11 | Magnet Off/OHLC, viewport/replay stability, cleanup/no errors | Snap/unsnap/no-future and lifecycle browser evidence |

## Milestones

1. **Domain/schema:** complete union, strict validator, fixtures, pure geometry/magnet rules, and focused tests.
2. **Repository/persistence:** current local and backend legacy migration, backup/quarantine/idempotence, serialized local/backend transaction, conflicts/failures, and tests.
3. **Provider:** single primitive/listener ownership, rendering/hit testing, all creation/edit/cancel semantics, semantic snapshot, and cleanup tests.
4. **Controller/UI:** tool-aware toolbar/editor, text entry, magnet control, exact history/persistence wiring, keyboard/clear/error UX.
5. **Browser closure:** additive D-01–D-11 UAT, seven retained gaps genuinely green, required screenshots, remount/replay/viewport/no-future/error evidence.
6. **Final gates:** full verification, DB hash, diff/assertion self-review, ExecPlan completion, stop at Reviewer gate.

## Risks and stop conditions

- Logical-time conversion may return null or duplicate time anchors: reject invalid points, use official `coordinateToTime`/`timeToCoordinate`, preserve preview, and test duplicates/nulls.
- Dense overlapping geometry can make selection ambiguous: deterministic reverse document order, handle priority, and fixed CSS-pixel tolerances.
- Backend endpoint lacks atomic CAS: use serialized preflight/verify policy; stop if live evidence shows it cannot prevent a stale/divergent overwrite without a contract change.
- Fib/Text round-trip or schema mismatch: stop for Reviewer schema decision rather than add provider state or reinterpret v1.
- Magnet must never access future candles: provider receives only current chart candle data; stop if a required snap needs undisclosed data.
- Stop if official APIs cannot provide reliable multi-anchor conversion/hit/cleanup, or if implementation would require a dependency, fork/private API, chart replacement, acceptance weakening, or Batch 4 work.

## Rollback

- Restore the approved Batch 1 Horizontal provider/controller/toolbar and remove Batch 3-only domain/geometry/migration/UAT additions. Existing valid Horizontal identity records remain schema-v1-compatible.
- Backend stores opaque JSON and receives no schema/database migration; rollback may restore from `sumi:drawing-legacy-backup:v1:*` or continue using local Horizontal documents.
- Chart primitives and previews are ephemeral and are detached on provider destroy; no chart-native migration is needed.

## Exact verification commands

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

## Progress log

- 2026-07-18: read all required authority, decision, schema, review, ExecPlan, drawing implementation/test/UAT sources in the mandated order.
- 2026-07-18: inventoried the dirty checkout, branch/HEAD/tag, preserved inputs, exact accepted assertion IDs, and production DB hash.
- 2026-07-18: reran technical baseline successfully. The first sandboxed UAT launch was unable to bind localhost (`EPERM`) and produced no product result; reran the same isolated harness with localhost permission successfully.
- 2026-07-18: isolated baseline UAT `2026-07-18T00-53-20-030Z` reproduced 79 pass / 7 authorized drawing gaps / `blockingFailed: 0` with zero runtime and indicator-request errors.
- 2026-07-18: wrote this ExecPlan, interaction/magnet/schema/migration/persistence contracts, affected modules, D mapping, rollback, risks, and exact gates before product code changes.
- 2026-07-18: implemented and focused-tested the complete schema-v1 union, strict validation, pure geometry/hit testing, visible-candle-only OHLC magnet, one-primitive provider, repository migration/backup/quarantine/conflict rules, serialized backend/local controller transactions, seven-tool UI, and additive browser assertions.
- 2026-07-18: removed the legacy production drawing renderer from Replay composition so the Sumi domain/controller/provider path is the sole drawing authority; retained legacy parsing only at the repository migration boundary.
- 2026-07-18: browser-tested create/cancel/select/body move/anchor edit/settings edit/delete/clear/undo/redo/reload/pan/zoom/replay/remount for every required tool, including Fib direction/labels, Text entry/edit/cancel, magnet Off/OHLC, non-price-pane isolation, ten provider remount cycles, and compact viewport behavior.
- 2026-07-18: completed the pre-review Batch 3 full product gate at `test-results/product-uat/2026-07-18T01-26-43-220Z`: backend 75 passed / 1 skipped, frontend 17 files / 70 tests, lint/build pass, browser UAT 174 passed / 0 failed / 0 blocking failed.
- 2026-07-18: appended the Reviewer-hardening plan before product edits, then closed B3-R01–B3-R06 with a schema/runtime contract, one serialized hydration/migration/command authority, exact gesture cancellation, official-coordinate rejection, rightward-Ray/four-corner/multiline geometry, the external responsive inspector, and additive focused/browser proof.
- 2026-07-18: completed the hardening full product gate at `test-results/product-uat/2026-07-18T03-15-14-250Z`: backend 75 passed / 1 skipped, frontend 18 files / 85 tests, lint/build pass, browser UAT 216 passed / 0 failed / 0 blocking failed.
- 2026-07-18: manually reviewed fresh 1440x1000 and 1280x800 evidence for four Rectangle handles, multiline Text bounds, the two-anchor inspector, and compact containment; self-reviewed assertion preservation, diff whitespace, provider/request/runtime errors, and production DB hash.

## Decision log

- Reconcile schema v1 rather than version it: approved schema already defines the full discriminated union and the shipped Horizontal document is a compatible subset.
- Keep local identity state as canonical working authority and the opaque backend JSON as required durable mirror; use serialized preflight/echo verification and surface divergence because backend atomic CAS is unavailable.
- Use a single ordered document primitive and reverse-order deterministic hit testing; no primitive/listener per drawing.
- Use 10px x/y OHLC thresholds with deterministic candle/field tie-break and visible-candle-only input.

## Deviations and completion evidence

- **D-01:** pass — seven discoverable tool buttons, active/tooltips/help, Select-only behavior, tool switching, and cancel semantics are browser asserted.
- **D-02:** pass — Horizontal create/select/body/handle edit/delete/history/persistence/reload behavior remains green.
- **D-03:** pass — finite Trendline creation, body translation, endpoint edit, selection, history, and reload are green.
- **D-04:** pass — right-extending Ray direction, body/endpoint edits, selection, history, and reload are green.
- **D-05:** pass — Rectangle normalized rendering, border/fill selection, corner/body edits, history, and reload are green.
- **D-06:** pass — directional Fibonacci anchors, standard seven ratios/prices/labels, direction reversal, edits, history, and reload are green.
- **D-07:** pass — Text/Note placement, validated entry, visible edit, move, cancel/empty suppression, history, and reload are green.
- **D-08:** pass — handle-first/topmost hit policy, price-pane-only interaction, exact UI/keyboard delete, and one-command Clear All are green.
- **D-09:** pass — create, body move, anchor edit, settings/Text/Fib edit, delete, clear, undo, and redo restore exact semantic states and persist once per completed command.
- **D-10:** pass — strict canonical local/backend equality, identity isolation, legacy migration, backup/quarantine/idempotence, failure rollback, conflicts, and reload without duplication are covered by focused and browser evidence.
- **D-11:** pass — Off/OHLC magnet, no-future visible-candle input, pan/zoom/replay reprojection, ten mount/unmount cycles, compact viewport, and zero page/console/provider errors are green.
- Pre-review focused drawing suite: 4 files / 29 tests passed. Hardening focused suite: 5 files / 44 tests passed. Final complete frontend suite: 18 files / 85 tests passed. Final backend suite: 75 passed / 1 skipped. Lint and production build passed.
- Final machine-readable result: `test-results/product-uat/2026-07-18T03-15-14-250Z/results.json` — 216 passed / 0 failed / 0 blocking failed; failed IDs `[]`; runtime errors `0`; indicator request failures `0`; provider errors `0`. Runtime logs: `/var/folders/cq/17wktb557fq87zb4pczl4lh00000gp/T//sumi-product-uat.j7D3ro`.
- Required reviewed evidence in `test-results/product-uat/2026-07-18T03-15-14-250Z/` includes `07-compact-1280x800.png`, `08-fibonacci-selected.png`, `09-text-edited.png`, `10-all-tools-selected.png`, `11-all-tools-reloaded.png`, `12-all-tools-zoom-replay.png`, `13-rectangle-four-handles.png`, `14-multiline-text-selection-bounds.png`, and `15-two-anchor-inspector-1440x1000.png`. The professional view is 1440x1000 and compact evidence is 1280x800; the wide inspector capture is reset to its top so tool name, stable ID and first anchors are visible.
- Assertion self-review compared the Reviewer baseline `2026-07-18T01-37-13-462Z` (174/174) with the final 216-check run: missing/renamed baseline IDs `[]`, duplicate final IDs `[]`, and 42 additive hardening IDs. Existing names and blocking classification were not changed or weakened; added waits observe real asynchronous persistence/rendering rather than bypass assertions.
- Production DB SHA-256 after all gates is `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`, identical to the before-hash. No backend schema, dependency, chart base, telemetry, or production DB change was introduced.
- Deliberate compatibility detail: the Text tool keeps HTML `title="Text"` for the accepted tooltip contract and exposes the fuller accessible label `Text / Note`.
- Known limitation for Reviewer inspection: the existing opaque backend endpoint cannot supply atomic database compare-and-swap. Batch 3 serializes a single client writer, preflights the expected mirror, rejects every canonical divergence, verifies the echoed document, rolls local state/history back on failure, and stops further writes with a visible conflict. It does not claim cross-client atomicity or silently overwrite a divergent document.
- Offline/backend failure retains the last committed local recovery document but does not report a new semantic command successful; commits resume only after backend reconciliation. Body movement intentionally preserves geometry deltas and does not snap individual anchors; only creation and endpoint edits use magnet, per the approved contract.
- **B3-R01: pass for Reviewer inspection.** JSON schema, TypeScript discriminants/runtime validation, fixtures, migration and repository now enforce price-pane-only documents, exact ordered seven-level Fibonacci semantics, nonblank trimmed Text up to 2000 characters, exact geometry/tool and anchor-count mappings, valid calendar dates, nonempty optional colors and strictly rightward Rays. A dependency-free schema drift test reads the JSON schema. Valid Horizontal v1 byte semantics remain unchanged; no previously supported canonical nonstandard Fib/pane record was discovered.
- **B3-R02: pass for Reviewer inspection.** Hydration, legacy promotion and all semantic commands share one identity queue. Readiness waits for verified backend echo; migration failure remains error/paused with backup; malformed or ambiguous remote input is quarantined/conflict-blocked; queued mutations derive from the latest committed document. Focused tests prove two immediate edits produce revisions 1 then 2 and conflict/failure preserves UI, canonical local state, backend mirror and history equality. Cross-client database atomicity remains explicitly unavailable.
- **B3-R03: pass for Reviewer inspection.** Tool switch, Escape/Cursor, `pointercancel`, lost capture, unmount and idempotent `destroy()` share exact rollback/cleanup. A switch during drag restores the before drawing/selection; a switch during preview discards it. Null/invalid official time/price conversions and unavailable bounds reject without fallback, commit, capture, scroll, listener, revision or history residue.
- **B3-R04: pass for Reviewer inspection.** Ray create and pointer/inspector edits require a strictly later second anchor. Rectangle exposes and hit-tests all four corners with deterministic mixed time/price field mapping while retaining two canonical anchors. Text uses a shared render/hit/select layout with explicit newlines, a 240px maximum rendered width and at most 12 displayed lines with ellipsis; canonical accepted content remains intact. One primitive and one six-listener set remain the lifecycle invariant.
- **B3-R05: pass for Reviewer inspection.** The icon rail is compact and the Sumi-owned labelled inspector resides in the right details rail with stable short/full ID, anchors, applicable line/fill/Text fields, visibility, lock and Fibonacci direction/reverse. Apply is one serialized semantic command; Cancel/invalid drafts do nothing; focused input keyboard events stay isolated. Wide and compact containment are browser asserted and manually reviewed.
- **B3-R06: pass for Reviewer inspection.** Focused tests cover schema compatibility, migration queue/failure/quarantine, rapid edits, rollback equality, cancellation/coordinate failures, Ray direction, four corners, multiline bounds and destroy. Browser proof uses real pointer gestures for both Trendline/Ray/Fib endpoints, every Rectangle corner, Text anchor and all bodies; records exact before/after anchors and one revision/history command; verifies inspector persistence/history/reload/keyboard/containment and ten-remount single-response behavior.
- No product-ready declaration, commit, push, merge, retag, branch/worktree action, backend contract change, or Batch 4 work was performed. Batch 3 stops here for Reviewer/Orchestrator diff and evidence inspection.

## Reviewer hardening re-inspection — 2026-07-18

Status: **RETURNED FOR SECOND BOUNDED CLOSURE. Batch 3 remains open; Batch 4 remains unauthorized.**

The Reviewer independently passed direct frontend tests/lint/build, `verify-v2.sh`, standalone product UAT, and `verify-product.sh`. Independent artifacts are `test-results/product-uat/2026-07-18T03-49-11-695Z` and `test-results/product-uat/2026-07-18T03-50-26-303Z`; both record 216 passed / 0 failed / `blockingFailed: 0` with zero runtime/provider/request errors. The production DB hash remained unchanged.

Happy-path hardening and B3-R05 are accepted, but B3-R07 through B3-R10 in `docs/reviews/BATCH_3_REVIEW_2026-07-18.md` remain P1 blockers:

- schema/runtime v1 counterexamples and a constant-only drift test;
- no reconciliation after a PUT may have committed before an error or mismatched echo;
- partial body movement when only some official anchor conversions succeed;
- UAT checks any anchor change rather than exact endpoint/corner/body field and delta semantics.

The next DEV pass is limited to those four findings. It must preserve all 216 current assertion IDs and every accepted Batch 1–3 behavior, record the second-closure plan before product edits, and stop again at the Reviewer gate. Authority: `docs/dev-prompts/BATCH_3_SECOND_REVIEW_CLOSURE_PROMPT.md`.

## Second Reviewer closure completion evidence — 2026-07-18

Status: **IMPLEMENTED AND VERIFIED; READY FOR REVIEWER INSPECTION. Batch 3 is not self-approved and Batch 4/5 remain unauthorized.**

### Finding closure

- **B3-R07: pass for Reviewer inspection.** The v1 Draft 2020-12 schema now expresses strict nested types, nonempty optional colors, exact discriminated shapes, and real Gregorian dates including leap/century rules. `drawingContractCorpus.ts` and the shared 20-case JSON corpus are consumed by both an actual AJV 8 Draft 2020-12 validator and the Sumi runtime semantic validator. Structural and supplemental results are asserted independently for valid all-tool/Horizontal documents, unknown/future/mismatched shapes, Fib/Text limits, impossible dates, Ray ordering, duplicate IDs, noncontiguous order, and repository identity. Supplemental invariants remain explicitly named: strictly rightward Ray, unique IDs, contiguous unique order, and requested session/symbol identity.
- **B3-R08: pass for Reviewer inspection.** Migration, ordinary commands, undo, and redo use the same serialized indeterminate-write reconciliation path. After a dispatched PUT failure or mismatched echo, one GET classifies remote as intended, prior, divergent, or unreadable: intended commits UI/local/history exactly once; prior restores exact prior state; divergent/unreadable preserves prior/intended/observed recovery evidence, exposes `indeterminate`, and blocks later writes until reload/reconciliation. Focused tests model PUT committing before throwing and remote mutation with mismatched echo; UAT proves intended, divergent, unavailable, and reload recovery states against exact UI/local/backend documents.
- **B3-R09: pass for Reviewer inspection.** Multi-anchor body conversion is atomic. All translated anchors must pass official time/price conversion and real-date/positive-price validation before preview publication. Any single time or price failure rolls back the exact document, releases capture, restores scrolling, leaves selection/revision/history/local/backend unchanged, and cannot commit on pointerup. Successful Trendline, Ray, Rectangle, and Fibonacci body moves preserve equal logical candle-index deltas and exact equal price deltas.
- **B3-R10: pass for Reviewer inspection.** Existing real-pointer endpoint/corner/body checks now assert nonzero x/y gestures, exact changed and unchanged canonical fields, strictly rightward Ray, and equal logical-index/price body deltas. Eight additive blocking IDs prove partial time/price rejection, the shared schema/runtime corpus, commit-then-error and mismatched-echo acceptance, divergent/unavailable blocking, and reload recovery. No existing ID was removed, renamed, skipped, weakened, or made nonblocking.

### D-01–D-11 retained acceptance

- **D-01–D-03: pass** — seven-tool discoverability/Select semantics, complete Horizontal lifecycle, and finite Trendline lifecycle remain green, including exact gesture/history/persistence behavior.
- **D-04–D-07: pass** — strictly rightward Ray, four-corner Rectangle, directional seven-level Fibonacci, and validated multiline Text/Note creation/edit/move/cancel/reload remain green.
- **D-08–D-09: pass** — deterministic handle/topmost selection, exact delete/clear, and one-command create/edit/delete/clear/undo/redo state restoration remain green.
- **D-10: pass** — canonical identity/local/backend equality, migration/backup/quarantine, serialized persistence, indeterminate reconciliation, recovery evidence, and reload behavior are covered by focused and browser evidence.
- **D-11: pass** — visible-candle-only Off/OHLC magnet, replay/pan/zoom stability, provider cleanup/remount, compact containment, and zero runtime/page/console/provider/request errors remain green.

### Exact implementation and test surface

- Contract/domain: `docs/decision-packs/sumi-drawing-document-v1.schema.json`, `frontend/src/features/drawings/drawingDomain.ts`, new `drawingContractCorpus.ts`, new `__fixtures__/drawing-contract-corpus.json`, and `__tests__/drawingDomain.test.ts`.
- Persistence: `DrawingRepository.ts`, `useDrawingWorkspaceController.ts`, `__tests__/useDrawingWorkspaceController.test.ts`, and the narrow controlled UAT response-header handling in `frontend/src/api/replayApi.ts`.
- Provider/ownership: `SumiPrimitiveDrawingProvider.ts`, its focused test, and `CandleChart.tsx`; the dormant legacy `DrawingToolRegistry` instantiation was removed from production composition so it cannot be a second mutable renderer authority.
- Browser proof: `ReplayWorkspace.tsx` exposes the computed corpus result only as a hidden machine-readable output, and `scripts/product-uat.mjs` uses the same schema/corpus with AJV and controlled deterministic persistence/conversion routes.
- Dependency deviation from the initial no-new-production-dependency preference: dev-only `ajv@8.17.1` was added to `frontend/package.json`/lockfile for genuine Draft 2020-12 validation. It is absent from production imports/bundle; AJV is MIT and the newly resolved `fast-uri` dependency is BSD-3-Clause. `npm audit` reports one pre-existing moderate advisory; no broad auto-fix was applied.

### Verification and retained evidence

- Focused second-closure suite: 3 files / 57 tests passed. Complete frontend suite: 18 files / 107 tests passed. Backend: 75 passed / 1 skipped. Frontend lint, TypeScript/Vite production build, `git diff --check`, and `node --check scripts/product-uat.mjs` passed.
- `./scripts/verify-v2.sh` passed. Standalone isolated UAT `test-results/product-uat/2026-07-18T05-47-56-985Z/results.json` passed 224/224. `./scripts/verify-product.sh` passed with final machine-readable artifact `test-results/product-uat/2026-07-18T05-51-53-510Z/results.json`: 224 passed / 0 failed / 0 blocking failed; `runtimeErrors: []`, `indicatorRequestFailures: []`, and `providerErrors: []`. Final runtime logs: `/var/folders/cq/17wktb557fq87zb4pczl4lh00000gp/T//sumi-product-uat.20qDai`.
- Baseline comparison against `test-results/product-uat/2026-07-18T03-50-26-303Z/results.json`: baseline 216, final 224, missing IDs `[]`, duplicate IDs `[]`, changed pass values `[]`, failed IDs `[]`, renamed IDs `[]`, and changed blocking classifications `[]`. The classifier is unchanged and every new ID uses the existing blocking `batch3.*` namespace. Added IDs are `batch3.second-closure.partial-body-time-rejected`, `partial-body-price-rejected`, `schema-runtime-corpus`, `commit-then-error-intended`, `mismatched-echo-intended`, `divergent-remote-blocked`, `unavailable-remote-blocked`, and `blocked-recovery-reload`.
- The first post-change UAT artifact `2026-07-18T05-45-04-718Z` exposed expected controlled network-abort console noise. The seam was narrowed to successful responses with explicit UAT-only headers so the same post-dispatch failure states are exercised without manufacturing page errors; no assertion or blocking rule was weakened. Both subsequent isolated runs are fully green.
- Manually reviewed final 1440x1000 screenshots `13-rectangle-four-handles.png`, `14-multiline-text-selection-bounds.png`, and `15-two-anchor-inspector-1440x1000.png`, plus `07-compact-1280x800.png`. Rectangle handles, multiline Text bounds, and inspector fields are visible and contained; the chart and replay/order controls remain usable without blocking overlap. The intentionally dense all-tools capture and the previously accepted compact indicator-strip horizontal overflow remain nonblocking and were not reclassified.
- Production DB SHA-256 after all gates is `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`, identical to the before-hash. All UAT/backend writes used isolated temporary databases. HEAD remains `108aa5dc0e26994607836e2b3b33f482e3791b4e`; protected tag `v2.0.0-rc2` remains at `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`. No branch/worktree, stage, commit, push, merge, reset, clean, checkout, tag, backend contract/schema, telemetry, chart-provider, private API, Batch 4, or Batch 5 action was performed.
- Known limitation retained for Reviewer inspection: the opaque backend endpoint still has no atomic database compare-and-swap. This closure resolves ambiguous outcomes for the serialized single-client writer and blocks on divergent/unreadable remote evidence; it does not claim or provide cross-client atomicity.

## Final Reviewer closure — 2026-07-18

Status: **APPROVED AND CLOSED for D-01–D-11 and the Batch 3 Drawing MVP.**

The Reviewer independently closed B3-R07–B3-R10 after code/test/evidence inspection and fresh verification. Focused closure tests passed 57/57; the full frontend passed 107/107 with lint/build green; backend passed 75 with 1 skip; `verify-v2.sh`, standalone UAT, and `verify-product.sh` passed. Independent UAT artifacts are `test-results/product-uat/2026-07-18T06-23-17-806Z` and `test-results/product-uat/2026-07-18T06-24-40-732Z`, each with 224/224, zero blocking failures, and empty runtime/provider/indicator-request error arrays.

All 216 previously accepted IDs remain present exactly once with unchanged pass/blocking semantics; eight blocking second-closure IDs are additive. The Reviewer visually inspected the fresh 1440×1000 Rectangle/Text/inspector evidence and 1280×800 compact evidence. `git diff --check` passed, `backend/sumi.db` remained at SHA-256 `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`, HEAD remained `108aa5dc0e26994607836e2b3b33f482e3791b4e`, and `v2.0.0-rc2^{}` remained `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.

The accepted limitation is unchanged: there is no cross-client database CAS on the opaque endpoint. Approval covers serialized single-client reconciliation and safe blocking of divergent/unavailable outcomes, not multi-client atomicity. This is Drawing MVP approval only, not product-complete or release-ready approval. Batch 4 is authorized only by `docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md`; Batch 5 remains unauthorized.
