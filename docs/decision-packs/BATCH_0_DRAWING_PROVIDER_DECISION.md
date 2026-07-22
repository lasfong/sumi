# Batch 0 drawing-provider decision pack

- Date: 2026-07-15
- Decision owner: Reviewer/orchestrator
- DEV recommendation: **Do not approve either community provider for production integration. Keep Lightweight Charts v5 and approve a Sumi-owned primitive-provider fallback only after Reviewer review.**
- Status: **Reviewer approved Option 1 with a bounded Batch 1 gate.** Neither community provider is approved for production. Batch 1 may implement only the `SumiPrimitiveDrawingProvider` boundary and one horizontal-line vertical slice under the stop/go conditions below.

## Decision summary

`deepentropy/lightweight-charts-drawing` is the better license and packaging shape in principle, and its browser spike was stable, but it fails mandatory product behavior: dragging a drawing body does not move it, specialized Text/Fibonacci settings disappear from exported JSON, and snapping is an unimplemented stub. Its repository also has no license file even though `package.json` says MIT and GitHub metadata reports no detected license.

`difurious/lightweight-charts-line-tools-core` has a materially stronger interaction and serialization design, including intended body/anchor interaction, complete option export, magnet configuration, and an explicit `destroy()`. It still fails the adoption gate: Escape left a one-anchor orphan in browser interaction, move/edit behavior was not proved, there is no native undo/redo or keyboard delete, required tools span five MPL-2.0 packages, and every package's `test` script is a deliberate failure saying no tests are specified. The official React test app also emits React DOM-prop console errors on every mount; this is evidence that the supplied integration surface is not clean, but is not by itself proof of a core-provider lifecycle leak.

Neither candidate may be connected to Sumi's production Replay route in Batch 0. Do not change the base chart library. The recommended fallback is `SumiPrimitiveDrawingProvider`, implemented with official Lightweight Charts primitives behind the contract below. Estimated effort is 6–10 person-weeks for the D-01–D-11 MVP; Batch 1 should integrate only one vertical tool after the Reviewer accepts this direction.

## Exact candidate inventory

| Candidate | Audited revision/version | Install model | License evidence | Maintenance/test signal |
| --- | --- | --- | --- | --- |
| deepentropy | npm `0.1.1`, tag `778f1e5cf3d62c2499dd4c686a00ab66bb01c44f`; HEAD `5f2afc335028d6a188ce0a50361056518c84cf72` | npm tarball works; Git SHA install is broken because package exports `dist/` but has no `prepare` and the Git artifact contains no `dist/` | `package.json`: MIT; repository has no `LICENSE`; GitHub license detection: null | Created 2025-12-18; 74 stars/26 forks/2 open issues on audit date; no test/spec files or test script; one post-release fix at HEAD |
| difurious core | `1.1.1`, `167a83cf8702e35b4cfbe7beb0dafec94e800a71` | Git dependency; `prepare` builds Rollup + TypeDoc | MPL-2.0 file present and detected | Created 2025-12-20; 72 stars/18 forks/0 open issues; `test` exits 1 with “no test specified” |
| difurious lines | `1.1.0`, `edb2a6ce00c8bbbe6f19e8469e378350efe6013f` | Separate Git dependency | MPL-2.0 | No automated tests |
| difurious rectangle | `1.1.0`, `8c229e62852f72936dbef5fdc198e321b9a85cc3` | Separate Git dependency | MPL-2.0 | No automated tests |
| difurious Fibonacci | `1.1.0`, `248b46813b44dd2dbe1576a9bce67ddf39d1338f` | Separate Git dependency | MPL-2.0 | No automated tests |
| difurious text | `1.1.0`, `3a64a17814c85a98cccccce1f5a014ef8e18a091` | Separate Git dependency | MPL-2.0 | No automated tests |
| difurious React test app | `0.2.0`, `e306d8f6edf85cdd06b6e3a9096d92540cad708f` | 334 installed packages across the full demo ecosystem | No repository license detected | 5 stars; has a test-named file but no test command; emits React console errors |

MPL-2.0 permits a larger work under different terms but requires Covered Software and modifications to Covered Software to remain available in Source Code Form with notices. Reviewer/legal approval is required before accepting that distribution and source-availability obligation.

## Bundle and install impact

| Measurement | Result |
| --- | ---: |
| deepentropy published ESM artifact | 326,630 bytes before consumer minification |
| deepentropy isolated React + Lightweight Charts spike | 578.54 kB minified / 147.08 kB gzip |
| difurious required provider minified artifacts (core + four companion packages) | 132,280 bytes, excluding React and Lightweight Charts |
| difurious full official React test app | 983.70 kB minified / 240.75 kB gzip |

Both builds triggered Vite's chunk-over-500-kB warning. Deepentropy exports all 68 tools from one entry point; the spike did not demonstrate a reliable required-subset bundle. Difurious is modular, but Git installs build packages locally and its runtime dependency lists include TypeDoc, increasing install surface.

## D-01–D-11 scorecard

| ID | deepentropy `0.1.1` | difurious revisions above | Decision consequence |
| --- | --- | --- | --- |
| D-01 | PASS: all required tools created/rendered | PASS: all required tools created/rendered; one canceled orphan remained | Tool inventory alone is insufficient |
| D-02 | PASS with Sumi spike controller handling Escape/Cursor | FAIL: Escape retained an incomplete one-anchor TrendLine | Adapter would need to detect/remove incomplete tools |
| D-03 | PASS: selection and handles observed | PARTIAL: selectable design and visuals observed; automation did not prove a stable selected-state contract | Contract tests still required |
| D-04 | PARTIAL: anchor edit passes; body move fails | FAIL in browser attempt: neither body nor anchor state changed | Mandatory rejection |
| D-05 | PASS through Sumi UI/keyboard mapping | PARTIAL: UI removal works; core explicitly omits Delete key | Sumi must own keyboard/UI commands |
| D-06 | PARTIAL: adapter snapshot undo/redo for delete demonstrated; no native history | FAIL: no native history | Sumi command history required regardless of provider |
| D-07 | PASS: pan, zoom, resize, replay append retained drawings | PARTIAL: official app renders multi-pane; full replay/resize sequence not proven | Must be re-proved in Batch 1/3 |
| D-08 | FAIL: base anchors/style round-trip, but Text/Fib-specific properties are omitted | PASS: full options export/import round-trip | deepentropy mandatory rejection |
| D-09 | PARTIAL: Fibonacci renders and has two anchors, but the automated check does not prove direction editing; custom semantics are lost on export | PASS for exported levels/options; interactive direction editing was not independently proved | Persistence remains decisive; interaction contract remains open |
| D-10 | FAIL: `applySnap` returns the unchanged anchor | PASS: browser changed magnet threshold and HEAD includes snapping fixes | deepentropy mandatory rejection |
| D-11 | PASS: 10 mount/unmount cycles, no page/console errors | UNPROVEN for core lifecycle: official React app emits two React DOM-prop console errors per mount | difurious adoption blocked pending a clean isolated proof |

Deepentropy machine result: `test-results/drawing-provider-spike/deepentropy/2026-07-15T14-25-40-462Z/results.json` (`20 pass / 3 fail`).

Difurious machine result: `test-results/drawing-provider-spike/difurious/2026-07-15T14-33-53-821Z/results.json` (`12 pass / 8 fail`). Some failures are explicit provider gaps; D-03/D-04 are conservatively failed because the browser attempt did not produce changed exported points. Its machine-readable D-11/runtime checks are valid for the official demo integration that was exercised, not a conclusive attribution to core-provider listener cleanup.

## Sumi-owned provider contract

Provider calls stay below `ChartWorkspace`; React components and persistence never import community-provider classes or raw JSON.

```ts
type DrawingTool =
  | 'select' | 'horizontal' | 'trendline' | 'ray'
  | 'rectangle' | 'fibonacci-retracement' | 'text';

type DrawingProviderEvent =
  | { type: 'created'; drawing: SumiDrawing }
  | { type: 'selection-changed'; drawingIds: string[] }
  | { type: 'change-started'; drawingIds: string[] }
  | { type: 'change-preview'; drawings: SumiDrawing[] }
  | { type: 'change-committed'; before: SumiDrawing[]; after: SumiDrawing[] }
  | { type: 'removed'; drawings: SumiDrawing[] }
  | { type: 'cancelled'; tool: DrawingTool }
  | { type: 'error'; code: string; message: string; cause?: unknown };

interface DrawingProvider {
  attach(context: DrawingProviderContext): void;
  setTool(tool: DrawingTool): void;
  cancel(): void;
  setMagnet(config: { enabled: boolean; thresholdPx: number }): void;
  getSelection(): string[];
  select(ids: string[]): void;
  update(drawing: SumiDrawing): void;
  remove(ids: string[]): void;
  clear(): void;
  replaceDocument(document: SumiDrawingDocumentV1): void;
  snapshot(): SumiDrawingDocumentV1;
  subscribe(listener: (event: DrawingProviderEvent) => void): () => void;
  destroy(): void;
}
```

`destroy()` must be idempotent and remove chart subscriptions, DOM listeners, primitives, previews, selection, and provider event listeners. Provider adapters may hold an ephemeral provider ID map, but provider IDs/raw JSON must never cross the adapter or persistence boundary.

## Versioned Sumi drawing state

The canonical JSON Schema is [sumi-drawing-document-v1.schema.json](sumi-drawing-document-v1.schema.json). Key rules:

- The document has an integer `schemaVersion`, optimistic `revision`, replay/session identity, and ordered drawings.
- Every drawing has a Sumi UUID, Sumi tool name, pane identity, explicit anchors, style, visibility/lock, and tool-specific geometry.
- Text content and Fibonacci levels/direction are first-class Sumi fields, not provider options.
- Provider-native blobs are forbidden. Unknown future Sumi fields are handled by schema-version migration, not round-tripped through a provider.
- Times are session dates (`YYYY-MM-DD`) for current daily replay. A future intraday schema version may add UTC epoch/timezone semantics; adapters must not silently reinterpret them.

## Provider event mapping

| Sumi event | deepentropy mapping | difurious mapping | primitive fallback |
| --- | --- | --- | --- |
| `created` | `drawing:added`, excluding preview IDs | `AfterEdit` after point count reaches tool requirement | interaction controller commit |
| `selection-changed` | selected/deselected events | SingleClick delegate + `getSelectedLineTools()` | hit-test selection store |
| `change-started` | adapter captures snapshot on pointer-down | adapter captures selected export on pointer-down | controller captures domain state |
| `change-preview` | `drawing:updated`, throttled for render only | internal motion; do not persist per frame | controller preview |
| `change-committed` | coalesce update events until pointer-up | `AfterEdit` + before snapshot | controller commit |
| `removed` | removed/cleared events plus before snapshot | removal commands plus before snapshot | command handler |
| `cancelled` | Sumi controller owns Escape/Cursor | Sumi must remove incomplete provider tool | controller owns cancel |
| `error` | adapter wraps exceptions/warnings | adapter wraps exceptions/dummy API detection | provider error boundary |

## Persistence and undo/redo ownership

`ReplayWorkspaceController` owns the live `SumiDrawingDocumentV1`. `DrawingRepository` owns backend/local persistence and revision conflicts. The provider adapter is a renderer/interaction engine only.

Persistence occurs after `created`, `change-committed`, `removed`, clear confirmation, undo, or redo. Pointer-move previews never write to the backend. Save requests are serialized per session and use the document revision to reject stale writes.

`DrawingCommandHistory` owns undo/redo with Sumi-domain commands (`CreateDrawing`, `ChangeDrawing`, `DeleteDrawing`, `ClearDrawings`). Each command stores minimal before/after Sumi state and selection. A drag produces one command, not one command per pointer move. Undo/redo updates the domain document first, then calls `replaceDocument`; provider-native history is not trusted.

Existing four-type records remain read-only in Batch 0. Batch 3 must add explicit migration fixtures and backup/restore verification before writing schema v1 to existing sessions.

The v1 schema is an approved domain-design input, not yet a production persistence contract. Batch 1 must add schema validation fixtures and contract tests for every supported state transition before any production write path is enabled. Duplicate drawing IDs/order and revision-conflict behavior must be enforced in domain/repository code because JSON Schema alone does not establish those document-level invariants.

## Target component/state architecture

```text
ReplayPage (composition only)
  -> ReplayWorkspaceController
       -> Replay query/API adapters (no-future-leak retained)
       -> WorkspaceStore
            -> SumiDrawingDocumentV1
            -> DrawingCommandHistory
       -> DrawingRepository (versioned persistence)
  -> ReplayWorkspace
       -> DrawingToolbar / SelectionToolbar
       -> ChartWorkspace facade
            -> LightweightChartsProvider
            -> DrawingProvider (adapter boundary)
                 -> recommended: SumiPrimitiveDrawingProvider
```

Batch 1 may establish this boundary and one horizontal-line vertical slice only. It must not implement the full Drawing MVP or migrate existing drawing records.

## Browser contract for Batch 1/3

Stable selectors must cover toolbar tools, active/cancel state, selected drawing controls, undo/redo, confirm-clear, chart workspace, serialized test state, and mount toggle. Every run captures:

- 1440×1000 required-tools screenshot;
- after-edit screenshot with visible handles;
- after pan/zoom/replay screenshot;
- after reload screenshot;
- machine-readable D-01–D-11 checks;
- console/page errors and mount/unmount iteration count.

## Primitive fallback estimate and risks

| Work | Estimate |
| --- | ---: |
| Provider boundary, hit-test/selection/command foundations | 8–12 days |
| Required line/ray/horizontal and rectangle primitives | 6–9 days |
| Fibonacci rendering/labels/direction and text editor | 7–10 days |
| Move/edit/magnet/pan/zoom/pane/replay lifecycle | 7–10 days |
| Persistence migration, undo/redo, browser contract hardening | 7–10 days |
| Total | 35–51 developer-days (about 6–10 person-weeks) |

Risks are custom hit testing, blank-space time mapping, mobile/retina coordinate precision, label collision, and lifecycle leaks. Mitigate with a small fixed tool set, official primitives only, a provider contract suite, and browser evidence as the authority. Reconsider the base chart library only if this estimate is rejected or an official-primitive spike proves a platform limitation.

## Exact verification commands

```bash
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
cd spikes/drawing-provider-deepentropy && npm install --force
cd spikes/drawing-provider-deepentropy && npm run build
node scripts/deepentropy-spike-uat.mjs
cd research_repos/lightweight-charts-line-tools-plugin-test-app && npm install
cd research_repos/lightweight-charts-line-tools-plugin-test-app && npm run build
node scripts/difurious-spike-uat.mjs
shasum -a 256 backend/sumi.db
```

## Reviewer gate and Batch 1 handoff

### Reviewer decision — 2026-07-15

**Option 1 is approved conditionally.** Keep Lightweight Charts v5. Do not integrate either audited community provider. Batch 1 is authorized only for a Sumi-owned provider boundary plus one production-quality horizontal-line vertical slice using official Lightweight Charts primitives.

This is not approval for the full 35–51 developer-day drawing roadmap. Batch 1 has a maximum implementation budget of 10 developer-days and must prove all of the following before another drawing tool is authorized:

- create, visibly select, move, edit, delete, Escape/cancel, clear with confirmation, undo, and redo for horizontal line;
- versioned Sumi-domain state, schema fixtures, save/reload persistence, and no provider-native JSON crossing the boundary;
- stable behavior through pan, zoom, replay advance, resize, reload, and ten mount/unmount cycles;
- deterministic 1440×1000 browser UAT, stable selectors, screenshots, serialized state assertions, and zero page/console errors;
- no future-candle leak, no production database mutation, and no drawing geometry or persistence logic added to `ReplayPage`;
- focused provider contract tests, frontend lint/test/build, `./scripts/verify-v2.sh`, `./scripts/run-product-uat.sh`, and reviewer-inspected evidence all passing for the scoped acceptance IDs.

Stop Batch 1 and open a base-chart-library reassessment ADR before Batch 2 if official primitives cannot provide reliable hit testing/coordinate conversion, the horizontal slice exceeds 10 developer-days, lifecycle cleanup cannot be made deterministic, or the implementation requires patching/forking Lightweight Charts internals. A maintained community fork remains unapproved unless a new spike closes every mandatory gap and separately resolves license/distribution posture.

The alternatives considered at this gate were:

1. Approve `SumiPrimitiveDrawingProvider` fallback and reframe Batch 1 around its first horizontal-line vertical slice (recommended).
2. Approve a maintained fork plus license posture and a re-spike that closes every mandatory failure.
3. Reject the fallback effort and commission a base-library reassessment ADR.

The choice is recorded above; Batch 1 may begin only after its own ExecPlan maps the bounded scope to acceptance IDs and exact verification commands. No acceptance criteria or product UAT assertions were changed or weakened in Batch 0.
