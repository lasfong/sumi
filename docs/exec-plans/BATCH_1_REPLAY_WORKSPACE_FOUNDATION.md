# Batch 1 — Replay workspace foundation and horizontal-line primitive slice

## Outcome

Deliver a production Replay workspace boundary and one complete Horizontal Line workflow using a Sumi-owned provider built on the official Lightweight Charts v5 series-primitive API. Cursor/Horizontal users can create, cancel, select, move, edit, delete, clear, undo/redo, persist, and restore horizontal lines without exposing provider-native state or changing existing backend records.

## Context and problem

- Batch 0 decision: `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`.
- Architecture direction: `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`.
- Product requirements: G-01, G-02, G-03, G-05; R-01 through R-05; scoped D-01 through D-08 and D-11 from `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`.
- `ReplayPage.tsx` currently owns replay queries, websocket updates, chart imperative calls, drawing state, drawing persistence, indicators, playback, and layout.
- `CandleChart.tsx` currently owns drawing geometry/input and delegates rendering to ordinary series/price lines. It has no selection, command history, provider contract, or schema-v1 persistence.

## Checkout provenance and preserved inputs

- User explicitly requires direct work on the current checkout: no branch/worktree creation or switch; no commit, push, merge, reset, clean, or retag.
- Branch/HEAD before Batch 1: `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2` remains `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Existing tracked Reviewer changes must remain intact: `.gitignore`, `docs/AGENTS.md`, `docs/INDEX.md`, `frontend/package.json`, `frontend/src/hooks/useWebSocket.ts`, and `frontend/vite.config.ts`.
- Existing untracked governance, harness, decision-pack, Batch 0 spike, and evidence files are preserved inputs. Batch 1 will not delete or rewrite their historical evidence.
- Production DB SHA-256 before implementation: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

## In scope

- Extract Replay application orchestration from `ReplayPage` into a controller hook and workspace composition components.
- Keep existing replay APIs, websocket behavior, markers, navigation, resume, trades, and backend indicator path.
- Introduce a chart facade and drawing-provider lifecycle contract.
- Implement `SumiPrimitiveDrawingProvider` for Cursor/Select and Horizontal Line only using official Lightweight Charts v5 primitives.
- Add selected rendering/handle, body drag, exact-price edit, UI/keyboard delete, confirmed Clear All, and domain-owned undo/redo.
- Add versioned schema-v1 domain validation, valid/invalid fixtures, round-trip tests, duplicate-ID/order invariants, revision-conflict behavior, and compatibility tests.
- Persist new schema-v1 documents through a local-first `DrawingRepository`; read existing backend four-type records as legacy display input only and never migrate/rewrite them.
- Extend deterministic product UAT with scoped assertions, serialized Sumi-domain state, retained screenshots/results/errors, 1440×1000 and 1280×800 layout evidence, and a 10-cycle lifecycle harness.

## Out of scope

- Trendline, Ray, Rectangle, Fibonacci, Text/Note, magnet/snapping, or full D-01.
- Indicator Manager or indicator UX changes.
- Broad Replay visual redesign, chart-library reassessment, community provider integration/fork, backend indicator changes, backend drawing schema/API changes, or legacy drawing migration.
- Batch 2 or any claim that the complete drawing system is delivered.

## Invariants

- Replay data remains session-scoped; the browser receives only candles through `current_index` and never slices a full future series.
- Backend `IndicatorEngine` remains authoritative.
- New drawing state contains no primitive instance, provider ID, provider JSON, or community-provider options.
- `backend/sumi.db` is never used by automated UAT; production hash must remain unchanged.
- Existing legacy backend drawing records are read-only. New schema-v1 writes use a separate local repository key and do not overwrite old records.
- One pointer drag commits one `ChangeDrawing` command; pointer-move previews do not persist or enter history.
- Provider destruction is idempotent and removes every DOM/chart subscription and attached primitive.
- Existing product assertions are retained without weakening.

## Current architecture

- `frontend/src/pages/ReplayPage.tsx`: queries, mutations, WebSocket, transforms, indicators, playback, drawing persistence, and layout.
- `frontend/src/components/chart/CandleChart.tsx`: chart lifecycle, input subscriptions, previews, and indicator imperative API.
- `frontend/src/components/chart/DrawingToolRegistry.ts`: legacy price-line/line-series renderer.
- `frontend/src/components/chart/SumiDrawingAdapter.ts`: legacy unversioned four-type array codec.
- `frontend/src/components/chart/WorkspacePersistence.ts`: version-1 local indicator/drawing envelope.
- `frontend/src/api/replayApi.ts` and backend drawing endpoints: opaque legacy `state_data` string.
- `scripts/product-uat.mjs`: deterministic baseline whose known Indicator Manager/full-tool failures must remain visible.

## Target design

```text
ReplayPage (route composition only)
  -> useReplayWorkspaceController (queries, replay/workspace orchestration)
  -> ReplayWorkspace (header/body composition)
       -> DrawingToolbar / DrawingSelectionToolbar
       -> ChartWorkspace facade
            -> Lightweight Charts series/pane management
            -> DrawingProvider contract
                 -> SumiPrimitiveDrawingProvider
       -> existing trade/journal panels

ReplayWorkspaceController
  -> SumiDrawingDocumentV1 domain state
  -> DrawingCommandHistory
  -> DrawingRepository (schema-v1 local persistence + revision CAS)
  -> legacy backend DrawingState read adapter (read-only)
```

`SumiPrimitiveDrawingProvider` owns coordinate conversion, primitive render/hit test, pointer interaction, selection visuals, and lifecycle. It emits Sumi-domain events only. The controller owns document mutation, command history, persistence serialization/revision, tool/cancel state, and selection. `ChartWorkspace` owns provider attach/destroy and delegates provider events upward. React UI never imports primitive classes.

## Expected affected modules

- Add `frontend/src/features/replay/useReplayWorkspaceController.ts` and `frontend/src/components/replay/ReplayWorkspace.tsx`.
- Refactor `frontend/src/pages/ReplayPage.tsx` to route-level composition.
- Refactor `frontend/src/components/chart/CandleChart.tsx` into the `ChartWorkspace` facade while preserving its public alias.
- Add drawing domain/provider modules under `frontend/src/features/drawings/`.
- Update `frontend/src/components/chart/DrawingToolbar.tsx` for scoped tools/history/selection controls and stable test IDs.
- Update `frontend/src/components/chart/workspaceTypes.ts`, `WorkspacePersistence.ts`, and focused tests only as needed for compatibility.
- Extend `scripts/product-uat.mjs`; retain every existing assertion and add scoped Batch 1 checks/artifacts.
- Add schema fixtures/tests under `frontend/src/features/drawings/__tests__/`.

The names follow the approved ownership model. The React controller is a hook rather than a class because it composes TanStack Query/WebSocket lifecycles; domain history/repository/provider remain framework-independent classes.

## Milestones

1. **Plan and baseline:** provenance, DB hash, ownership, compatibility, acceptance mapping, rollback, and exact gates recorded; current technical/product baseline understood.
2. **Domain boundary:** schema-v1 validation, fixtures, document invariants, command history, repository CAS, and legacy compatibility tests pass.
3. **Primitive provider slice:** official primitive renders Horizontal Line and passes create/cancel/select/move/edit/delete/clear/undo/redo plus idempotent lifecycle tests.
4. **Replay foundation:** `ReplayPage` is composition-only; existing replay, marker, indicator, navigation, resume, websocket, and trade behavior remain wired through controller/workspace boundaries.
5. **Browser evidence and gates:** focused scoped UAT passes with zero runtime errors and retained evidence; all required project gates pass; DB hash unchanged; self-review recorded; stop at Reviewer gate.

## Acceptance mapping

| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| G-01 | Full existing gates plus focused domain/provider/browser tests | `verify-v2`, `verify-product`, focused Vitest, product UAT |
| G-02 | Existing isolated UAT temporary DB harness retained | UAT runtime directory and unchanged production hash |
| G-03 | Results, screenshots, console/page logs retained for pass/fail | Timestamped `test-results/product-uat/*` |
| G-05 | Browser/localStorage/backend only; no telemetry/dependency added | Source audit and browser network evidence |
| R-01 | Existing session-scoped candle and indicator requests preserved | API payload length equals displayed/current bar; existing backend no-future tests |
| R-02 | Existing symbol/bar/OHLC plus explicit current date in workspace header | Browser header assertion/screenshots |
| R-03 | Existing previous/next/±5/play/pause/speed/keyboard controller wiring | Existing and focused browser interaction |
| R-04 | Query invalidation/WebSocket behavior preserved; provider document replaced without duplicates | Replay-advance drawing/domain assertion |
| R-05 | Zustand session resume, indicators, and new drawing repository restore | reload/resume browser equality assertion |
| D-01 subset | Cursor and Horizontal labels/tooltips only | Browser toolbar assertion explicitly marked subset |
| D-02 | Controller-owned active tool and Escape/Cursor cancellation | Browser create/cancel/orphan assertions |
| D-03 | Provider hit test, selection state, visible handle | Provider tests and selected screenshot |
| D-04 | Body drag and exact-price edit emit one committed domain change | History/provider tests and browser serialized state |
| D-05 | UI/keyboard delete; confirmed Clear All with undo | Browser assertions |
| D-06 | Domain `DrawingCommandHistory` for create/change/delete/clear | Unit and browser undo/redo assertions |
| D-07 | Primitive derives coordinates per render and survives viewport/replay/lifecycle changes | pan/zoom/resize/replay/reload browser assertions |
| D-08 | Schema-v1 document/repository validation and restore; no provider state | fixtures, round-trip, invariant/CAS tests, browser equality |
| D-11 | Idempotent detach; no stale listeners/errors across 10 cycles | lifecycle test and browser error logs |

## Persistence compatibility

- Schema-v1 is promoted only for the Horizontal slice after fixtures and semantic validation pass.
- New documents use the identity key `sumi:drawing-document:v1:<sessionId>:<encodedSymbol>` and include revision/session/symbol/order. The former session-only key remains a compatibility input. Repository saves compare-and-swap only against the requested identity and increment once per committed command.
- Existing backend `DrawingState.state_data` remains unchanged and read-only. Valid legacy horizontal records may render as compatibility overlays but are not imported into or silently rewritten as schema v1.
- Existing `WorkspacePersistence` indicator state remains readable. Its legacy drawing field remains tolerated but is not the authoritative new drawing store.
- Duplicate drawing IDs, non-contiguous order, invalid UUID/date/price/style/tool geometry, session/symbol mismatch, or stale revision are rejected before persistence.

## Rollback and compatibility

- Rollback is removal of the Batch 1 facade/controller/provider modules and restoration of their call sites; no database rollback or migration is required.
- Clearing the new localStorage schema-v1 key disables only Batch 1 horizontal documents. Legacy backend records are untouched.
- The `CandleChart` export alias and existing indicator imperative API remain during Batch 1 to bound regressions.
- If the primitive fails stop conditions, retain tests/evidence, disable the new provider path, update this plan, and open a Reviewer ADR rather than patching Lightweight Charts internals.

## Stop/go conditions

- **GO** only while official `ISeriesPrimitive` rendering, `priceToCoordinate`/`coordinateToPrice`, and stable container pointer events provide reliable hit testing/editing with deterministic teardown inside the 10-day budget.
- **STOP** if internals must be patched/forked, base library/dependency must change, cleanup is non-deterministic, no-future-leak cannot be retained, legacy records require migration/rewrite, scope must materially exceed Horizontal, or preserved Reviewer changes conflict unsafely.

## Verification commands

```bash
git diff --check
cd frontend && npm test -- --run src/features/drawings src/components/chart src/features/replay
cd frontend && npm run lint
cd frontend && npm run build
cd backend && ../.venv/bin/python -m pytest app/tests/test_replay_no_future_leak.py app/tests/test_api_integration.py -q
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

## Risks and mitigations

- Canvas hit precision after zoom/resize: use `priceToCoordinate` at event time and primitive `hitTest`, with tolerance asserted in browser tests.
- Drag conflicts with chart scrolling: capture pointer only after a selected line hit; restore chart interaction on commit/cancel.
- React stale closures/listeners: provider has one attach/destroy owner, stable event bridge, and idempotent cleanup tests plus 10 browser cycles.
- Revision races: repository compare-and-swap and serialized controller saves; stale revisions produce explicit conflicts.
- Legacy data loss: never PUT new schema-v1 into legacy endpoint in Batch 1.
- Broad refactor regressions: preserve aliases/contracts, keep backend APIs unchanged, and exercise navigation/resume/markers/websocket through full gates.
- Existing UAT is intentionally red for out-of-scope Indicator/full-drawing gaps: focused Batch 1 checks must be independently green without relabeling those gaps.

## Progress log

- 2026-07-15: DEV read all ten required canonical inputs in order, confirmed Reviewer authorization for the bounded official-primitive Horizontal slice, and did not start Batch 2.
- 2026-07-15: inventoried current dirty checkout and preserved Reviewer/governance/harness/evidence files. Recorded HEAD/tag and production DB hash.
- 2026-07-15: inspected current Replay/chart/drawing/API/UAT ownership and wrote this ExecPlan before feature coding.
- 2026-07-15: extracted `ReplayPage` to a five-line route composition surface and introduced `ReplayWorkspace`/`ReplayWorkspaceController` boundaries while preserving existing replay queries, indicators, markers, websocket, trades, resume, and navigation.
- 2026-07-15: implemented schema-v1 Horizontal domain validation, fixtures, duplicate/order invariants, revision-CAS repository, domain command history, provider event contract, and `SumiPrimitiveDrawingProvider` on the official v5 series primitive API.
- 2026-07-15: first focused browser run found a React updater side-effect revision conflict; persistence was moved outside state updaters. A later autoplay/reload run exposed a transient-symbol restore race; repository restore now loads by stable session identity before validating the resolved symbol.
- 2026-07-15: final browser result has 23/23 Batch 1 assertions PASS, zero blocking failures, and zero console/page errors. Full technical and product gates pass; production DB hash is unchanged.

## Decision log

- Use official Lightweight Charts v5 `ISeriesPrimitive` attached to the candlestick series; no dependency or chart-library change.
- Use a local schema-v1 repository for the new slice because the approved batch forbids legacy drawing migration/rewrite and the current backend endpoint has no schema/revision contract. The backend legacy record remains read-only.
- Treat Horizontal anchor `time` as the current replay candle date for domain validity; rendering/hit testing is price-based across the pane.
- Use a React controller hook for server/query/websocket orchestration and framework-independent classes for drawing domain/history/repository/provider ownership.
- Preserve every pre-existing product assertion and its pass/fail value in machine results. `blockingFailed` is a separate scoped gate: known Indicator Manager/full-tool/legacy-endpoint gaps remain explicit failures rather than being deleted or relabeled as passes.
- Superseded by Reviewer hardening: `ReplayWorkspaceController.tsx` now exports the non-visual `useReplayWorkspaceController` hook and typed view model; `ReplayWorkspace.tsx` owns the preserved header/chart/details composition. The controller owns orchestration but renders no page JSX.

## Completion evidence

- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 11 files/24 tests; lint/build PASS.
- `./scripts/verify-product.sh`: PASS; its run retained 23/23 scoped Batch 1 checks and zero runtime errors. A final focused rerun after navigation/autoplay coverage is `test-results/product-uat/2026-07-15T15-19-13-371Z/results.json`: 29 overall pass, 13 known out-of-scope failures, `blockingFailed: 0`, 23/23 Batch 1 PASS.
- Browser screenshots: `03-horizontal-created-selected.png`, `04-horizontal-moved-edited.png`, `05-horizontal-pan-zoom-replay.png`, `06-horizontal-reloaded.png`, and `07-compact-1280x800.png` in the final result directory.
- Production DB SHA-256 before/after: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Existing backend drawing `state_data` stayed `[]` during focused UAT by design; Batch 1 uses the versioned local repository and does not migrate/rewrite legacy records.
- Known limitations: only Cursor/Select and Horizontal are delivered; full D-01, D-09/D-10, Indicator Manager, backend schema-v1 synchronization, and legacy migration remain outside scope. The legacy chart registry remains read-only-render capable until a later migration batch.
- Historical pre-review deviation (resolved by hardening): the first pass left the controller as a rendering component. The 2026-07-16 hardening converted it to a non-visual hook and moved the actual composition to `ReplayWorkspace`. No dependency, backend contract, or acceptance criterion changed.
- Self-review confirmed no existing assertion was removed or changed to a fake pass. The harness now reports known failures separately from scoped blocking failures so the approved Batch 1 gate can pass without concealing future-batch gaps.
- DEV complete; stopped at Reviewer gate. No Batch 2 work, branch/worktree, commit, push, merge, reset, clean, or tag action was performed.

## Reviewer gate — 2026-07-16

Status: **RETURNED TO DEV; HARDENING IN PROGRESS. Batch 1 is not complete and Batch 2 is not authorized.**

The reviewer independently reproduced the technical and browser results, confirmed the functional Horizontal slice and unchanged production database, then found that the Replay extraction is largely a monolith relocation and that D-06/D-07 plus cancel/conflict/pane-boundary behavior are not proved by the current tests. Full findings and required closure are recorded in `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`. The bounded DEV continuation prompt is `docs/dev-prompts/BATCH_1_REVIEW_HARDENING_PROMPT.md`.

## Reviewer hardening plan — 2026-07-16

### Returned-gate outcome

Close B1-R01 through B1-R09 without adding tools, dependencies, backend writes, migrations, Indicator Manager work, or visual redesign. Preserve the accepted Horizontal behavior while replacing claimed-but-unproved architecture and UAT statements with real controller/view, transaction, cancel, pane, lifecycle, and viewport evidence.

### Hardening affected modules

- `frontend/src/components/replay/ReplayWorkspaceController.tsx`: become a non-visual view-model hook; no full-page JSX.
- `frontend/src/components/replay/ReplayWorkspace.tsx`: own the existing header/chart/details composition without redesign.
- `frontend/src/features/drawings/useDrawingWorkspaceController.ts`: transactional command execution and explicit provider cancel bridge.
- `frontend/src/features/drawings/DrawingCommandHistory.ts`: non-mutating peek/commit semantics for transactional undo/redo.
- `frontend/src/features/drawings/DrawingRepository.ts`: session/symbol identity collision enforcement and test seams.
- `frontend/src/features/drawings/DrawingProvider.ts`: reconcile the bounded provider interaction/lifecycle contract.
- `frontend/src/features/drawings/SumiPrimitiveDrawingProvider.ts`: cancel rollback, pointer cleanup, official price-pane bounds, lifecycle observability.
- `frontend/src/components/chart/CandleChart.tsx` and `workspaceTypes.ts`: isolate legacy drawing rendering, remove legacy input ownership, expose provider cancel and official pane/coordinate test contract through the facade.
- Focused tests under `frontend/src/features/drawings/__tests__/` and replay/chart tests as required.
- `scripts/product-uat.mjs`: replace the misleading aggregate undo check; add exact undo/redo, drag-cancel, non-price-pane, viewport interaction, replay rewind, resize alignment, and duplicate-event assertions while retaining all 13 out-of-scope failures.

### Hardening milestones

1. **B1-R01 controller/view boundary:** a hook returns a typed Replay view model and handlers; `ReplayWorkspace` renders the preserved UI; the controller contains no header/chart/detail JSX.
2. **B1-R03/R04 transactional lifecycle:** cancel restores pre-drag provider/domain state with no write/history/revision change; ordinary commit/undo/redo conflicts leave history/UI/persistence aligned and show controlled feedback.
3. **B1-R05/R07/R08/R09 provider boundary:** official `IPaneApi.getHTMLElement()` defines price-pane input bounds; only the primitive provider owns production interaction; the final minimal contract and lifecycle tests are explicit.
4. **B1-R02/R06 corrected evidence:** independent semantic undo/redo and post-pan/zoom/replay/resize hit/move checks pass, including non-price-pane rejection and ten-cycle event-count stability.
5. **Reviewer gate rerun:** focused/full technical gates and isolated product UAT pass with zero scoped/runtime failures, 13 retained product gaps, unchanged production DB hash, updated decisions/evidence, and no Batch 2 work.

### Hardening acceptance/finding mapping

| Finding | Acceptance | Closure evidence |
| --- | --- | --- |
| B1-R01 | Architecture target, R-01–R-05 | Non-visual controller hook test/source boundary; ReplayWorkspace composition; full regression gates |
| B1-R02 | D-06 | Exact ID/price assertions for undo/redo create, move, edit, UI delete, keyboard delete, and clear; one-drag revision delta |
| B1-R03 | D-02, D-04, D-06 | Provider/controller cancel test and browser drag-cancel state/revision/persistence equality |
| B1-R04 | D-06, D-08 | Commit/undo/redo stale-revision tests proving unchanged history and controlled feedback |
| B1-R05 | D-02–D-04, D-07 | Official `IPaneApi.getHTMLElement()` bounds; unit/browser rejection for Volume/RSI/MACD/CCI input |
| B1-R06 | D-07 | Machine assertions re-hit/reselect/move same ID after pan/zoom, replay advance/rewind, and both viewports |
| B1-R07 | D-08, D-11 | Final minimal provider contract decision plus contract event/lifecycle tests |
| B1-R08 | Architecture boundary, D-11 | Legacy registry renders read-only overlays only; legacy click/crosshair input path removed from production facade |
| B1-R09 | D-03, D-04, D-11 | Listener removal, no post-destroy events, hit/select, single commit, cancel cleanup, idempotent destroy tests |

### Hardening rollback and compatibility

- The first hardening pass retained the session-only schema-v1 key; B1-R11 final closure supersedes that local-storage detail with the documented identity-key compatibility policy. Backend legacy records remain untouched. Controller/view extraction remains a source-only ownership change with the current DOM/test contracts preserved.
- Provider rollback remains removal of the primitive interaction path while retaining legacy overlays. No database migration or cleanup is needed.
- Transaction failures restore/retain the last persisted document and unchanged history stacks. They never optimistically advance domain state.
- If official `IPaneApi.getHTMLElement()` cannot provide stable price-pane bounds across pane creation/resize, stop and document the chart-library reassessment condition rather than inspecting Lightweight Charts DOM internals.

### Hardening exact verification commands

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

### Hardening progress log

- 2026-07-16: DEV read the returned-gate prompt, all eight required sources, and the complete Reviewer report before code changes. Current HEAD and production DB hash remain unchanged; existing Reviewer/DEV working-tree changes are preserved.
- 2026-07-16: hardening scope, affected modules, milestones, acceptance/finding mapping, rollback, official-pane stop condition, and exact verification commands recorded before implementation.
- 2026-07-16: extracted the actual Replay composition into `ReplayWorkspace` and converted `ReplayWorkspaceController` to a non-visual hook/view-model boundary. Existing replay, indicators, marker, WebSocket, trade, journal, navigation, autoplay, and resume wiring remain in the controller model.
- 2026-07-16: made command application transactional. History exposes non-mutating peek plus accept operations; commit, undo, and redo advance history only after repository CAS succeeds. Stale commit/undo/redo tests verify document/history remain unchanged and feedback is controlled.
- 2026-07-16: wired Escape and Cursor through `ChartWorkspace.cancelDrawing()`. Provider cancel restores the exact pre-drag drawing, retains selection, releases capture, restores chart scrolling, emits no commit, and leaves repository/revision unchanged.
- 2026-07-16: bound pointer input to the official Lightweight Charts v5 price pane via `IPaneApi.getHTMLElement()`. Production legacy drawing input/crosshair ownership was removed while legacy overlays remain read-only.
- 2026-07-16: replaced the aggregate undo claim with independent create/move/edit/UI-delete/keyboard-delete/clear undo and redo assertions. Added cancel transaction, Volume/RSI/MACD/CCI rejection, pan/zoom hit-and-move, replay advance/rewind interaction, 1440×1000 and 1280×800 price alignment, reload equality, and single-event-after-ten-remount checks.
- 2026-07-16: final isolated product gate retained 41 passes, exactly 13 known out-of-scope failures, `blockingFailed: 0`, and zero console/page errors. All technical gates passed and production DB hash remained unchanged.

### Final provider ownership decision

The approved Batch 0 interface is intentionally narrowed at the provider boundary after implementation evidence. `DrawingProvider` owns only chart-engine concerns: active-tool projection, cancel, selection projection, document replacement for rendering, interaction snapshot, event subscription, and idempotent destruction. It emits `created`, `selection-changed`, `change-started`, `change-preview`, `change-committed`, and `cancelled` domain-shaped events.

Create/update/remove/clear, schema validation, command construction, undo/redo, revision CAS, persistence, and conflict feedback belong exclusively to the domain controller/repository. Adding provider mutation methods would create two owners for history and persistence, so the Batch 0 candidate contract's mutation/snapshot concepts are fulfilled by controller operations plus a read-only provider interaction snapshot, not mirrored as dead provider APIs. Consequences: provider replacement remains stateless with respect to persistence; failed transactions can restore the persisted document deterministically; future providers must implement the same minimal interaction/lifecycle contract, while domain commands remain provider-independent.

### Reviewer finding closure

| Finding | Closure |
| --- | --- |
| B1-R01 | Non-visual controller hook (426 lines) and actual workspace composition view (191 lines); `ReplayPage` remains route-only. |
| B1-R02 | Exact-ID/semantic-price create, move, edit, UI-delete, keyboard-delete, and clear undo/redo browser checks; one drag increments revision once. |
| B1-R03 | Escape/Cursor invoke provider cancel; unit and browser checks prove rollback, capture/scroll cleanup, retained selection, unchanged document/revision/persistence. |
| B1-R04 | Peek/save/accept history transaction; stale ordinary commit, undo, and redo tests prove no history mutation or uncaught handler error. |
| B1-R05 | Official `IPaneApi.getHTMLElement()` boundary plus provider and browser rejection outside price pane, including named Volume/RSI/MACD/CCI probes. |
| B1-R06 | Browser re-hit/reselect/move after pan/zoom and replay advance/rewind; machine price-coordinate alignment at both required viewport sizes. |
| B1-R07 | Final minimal contract/ownership decision above; focused lifecycle/event tests cover implemented contract. |
| B1-R08 | Legacy pending-click/crosshair input path removed from `ChartWorkspace`; legacy registry is render-only. |
| B1-R09 | Provider tests cover registration/removal, no events after destroy, idempotent destroy, pane rejection, hit/select, single drag commit, and cancel cleanup. |

### Hardening completion evidence

- Status: **HARDENING COMPLETE; READY FOR REVIEWER RE-INSPECTION. Batch 2 remains unauthorized.**
- `git diff --check`: PASS.
- Focused drawing/chart/replay Vitest: 6 files / 20 tests PASS.
- Full frontend Vitest through `verify-v2` and `verify-product`: 12 files / 31 tests PASS; lint/build PASS.
- Focused backend no-future/API: 8 tests PASS. Full backend: 75 passed / 1 skipped.
- `./scripts/run-product-uat.sh`: PASS; `test-results/product-uat/2026-07-16T14-05-20-205Z/results.json` has 41 pass, 13 retained failures, zero scoped/runtime failures.
- `./scripts/verify-product.sh`: PASS; final evidence is `test-results/product-uat/2026-07-16T14-07-00-972Z/results.json`, with the same 41/13/0 outcome and required screenshots.
- Production DB SHA-256 before/after: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Self-review: no existing product assertion was removed, weakened, or relabeled; the 13 out-of-scope failures remain visible. No dependency, backend contract, schema migration, legacy record rewrite, extra drawing tool, or Batch 2 work was introduced.

### Reviewer re-inspection status — 2026-07-16

Status: **RETURNED FOR TWO FINAL CLOSURE ITEMS.** Reviewer accepted B1-R01/R02/R04/R05/R06/R07/R08 and the tested B1-R09 behavior. B1-R03 remains incomplete for native `pointercancel`, which is currently routed through the commit handler. Repository session/symbol collision behavior also remains incomplete because a mismatched document is rejected on load but permanently blocks revision-0 writes for the new identity. See B1-R10/R11 in `docs/reviews/BATCH_1_REVIEW_2026-07-16.md` and use `docs/dev-prompts/BATCH_1_FINAL_CLOSURE_PROMPT.md`. Batch 2 remains unauthorized.

## Final closure plan — B1-R10/B1-R11 — 2026-07-16

Status: **FINAL CLOSURE IN PROGRESS; Batch 2 remains unauthorized.** No accepted B1-R01–R09 work is reopened.

### Scope and affected modules

- B1-R10 only: `SumiPrimitiveDrawingProvider.ts`, its focused provider test, and one additive scoped assertion in `scripts/product-uat.mjs`.
- B1-R11 only: `DrawingRepository.ts`, its focused domain/repository tests, and this persistence compatibility record.
- No controller/view refactor, new tool, dependency, backend contract, backend persistence, legacy migration, or visual change.

### Closure milestones and acceptance mapping

1. **B1-R10 / D-02, D-04, D-06, D-11:** give native `pointercancel` a dedicated rollback handler. It restores the drag-start drawing, clears drag state, safely releases capture only when still held, restores scrolling, retains selection/tool intent, publishes the final snapshot, and never emits `change-committed`.
2. **B1-R11 / D-08:** adopt identity-specific Sumi storage while preserving legacy compatibility. A same-session/different-symbol legacy document is moved non-destructively to its identity key before the new identity saves; same-symbol legacy documents continue to load; stale CAS for the same identity still fails.
3. Add focused provider/repository coverage and one real-browser `PointerEvent('pointercancel')` assertion that proves domain document, revision, localStorage, and history availability are unchanged.
4. Run all prompt-required gates against the isolated UAT database; retain machine results/screenshots and verify the production DB hash.

### Repository identity policy, compatibility, and rollback

- Canonical identity key: `sumi:drawing-document:v1:<sessionId>:<encodedSymbol>`.
- Existing session-only key `sumi:drawing-document:v1:<sessionId>` remains a compatibility input. Loading a matching symbol may promote it to the canonical identity key without data loss. Loading a different symbol never returns the old document.
- On the first save for a different symbol, the repository preserves the old valid legacy document under its canonical identity key (and a Sumi-owned collision backup key if that canonical key is already occupied), then allows the new identity to initialize at expected revision 0. It never imports old-symbol drawings into the new symbol.
- CAS reads and writes the requested identity only. Same-session/same-symbol stale revisions continue to conflict. Existing same-symbol legacy documents remain readable and are promoted safely.
- Rollback is source-only: the preserved legacy/canonical/backup values remain valid schema-v1 JSON and can be recovered by the recorded keys. No backend data or legacy backend drawing record is changed.

### Exact final-closure verification

```bash
git diff --check
cd frontend && npm test -- --run src/features/drawings
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

### Final closure progress and evidence

- 2026-07-16: added a dedicated native `pointercancel` listener/handler. It rolls provider and controller preview state back to the exact drag-start drawing, safely tolerates already-lost pointer capture, restores scrolling, retains selection/tool state, publishes the final snapshot, and emits no commit. Destroy removes the dedicated listener idempotently.
- 2026-07-16: promoted drawing persistence to `(sessionId, symbol)` identity keys. A mismatched session-only document is never loaded into the new symbol; before the new symbol saves at revision 0, the old document is preserved under its own identity key or a collision-backup key. Same-identity CAS behavior is unchanged.
- Focused drawings: `npm test -- --run src/features/drawings` PASS — 3 files / 16 tests. Provider coverage includes native rollback, zero commit, capture/scroll cleanup, and no event after destroy. Repository coverage includes mismatch rejection, new-identity initialization/save, old-identity recovery, same-symbol legacy promotion, and stale same-identity conflict.
- Lint and production build: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed / 1 skipped; frontend 12 files / 34 tests; lint/build PASS.
- `./scripts/run-product-uat.sh`: PASS — `test-results/product-uat/2026-07-16T14-33-26-951Z/results.json`, 42 pass / 13 retained product gaps / `blockingFailed: 0` / zero runtime errors.
- `./scripts/verify-product.sh`: PASS — final machine result `test-results/product-uat/2026-07-16T14-34-42-611Z/results.json`, also 42/13/0 with zero runtime errors. `batch1.native-pointercancel-transaction` proves document, identity-key localStorage, revision 22, and history availability unchanged; provider reports `dragging=false` and the same selected ID.
- Final screenshots retain required dimensions: `05-horizontal-pan-zoom-replay.png` is 1440×1000 and `07-compact-1280x800.png` is 1280×800 in the final result directory.
- Production DB SHA-256 before/after: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Harness deviation: the first pointercancel run exposed an incorrect `locator.evaluate` callback signature; the second exposed a pre-existing timing assumption after the tenth remount. Both were corrected additively without removing, renaming, weakening, or relabeling assertions. The final two isolated runs and full product gate are green.
- Self-review: B1-R10/R11 are the only reopened implementation areas. No accepted controller/view/provider work was refactored; no product assertion was removed; all 13 out-of-scope gaps remain visible; no dependency, backend contract, legacy backend migration, drawing tool, Indicator Manager, or Batch 2 work was introduced.

Status: **BATCH 1 FINAL CLOSURE COMPLETE; READY FOR REVIEWER INSPECTION. Batch 2 remains unauthorized.**

## Reviewer final closure — 2026-07-16

Status: **APPROVED AND CLOSED.** B1-R10 and B1-R11 passed code inspection, focused tests, and independent browser UAT. The independent final result is `test-results/product-uat/2026-07-16T14-37-46-450Z/results.json`: 42 passes, 13 retained out-of-scope failures, `blockingFailed: 0`, and zero runtime errors. Production DB SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

Batch 1 closes only the replay-workspace foundation and the official-primitives Horizontal Line vertical slice. The remaining Indicator Manager, full drawing MVP, and backend migration gaps are not accepted by this closure. Batch 2 may start only from `docs/dev-prompts/BATCH_2_INDICATOR_MANAGER_PROMPT.md`.
