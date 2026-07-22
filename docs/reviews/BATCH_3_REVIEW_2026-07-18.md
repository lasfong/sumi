# Batch 3 Reviewer gate — 2026-07-18

## Verdict

Status: **APPROVED AND CLOSED after two bounded closures and final independent verification. Batch 4 is authorized only through its dedicated DEV prompt.**

The initial Reviewer gate returned the Sumi-owned drawing implementation for the findings preserved below. The hardening and second closure subsequently resolved those findings; the final decision and independent evidence are recorded in “Final Reviewer closure — 2026-07-18.”

The following table is the **initial gate classification**, retained as review history rather than the current verdict.

Acceptance classification:

| Acceptance | Reviewer verdict | Reason |
| --- | --- | --- |
| D-01–D-02 | Provisionally accepted | Seven tools, active state, Escape/Cursor and basic creation are exercised in browser. Toolbar discoverability still needs the B3-R05 UX closure. |
| D-03–D-04 | PARTIAL | Selection and body moves work, but real endpoint/corner dragging is not proved; Rectangle exposes only two of four corners and tool-switch rollback is unsafe. |
| D-05–D-07 | Provisionally accepted | Exact delete/clear/history and ordinary pan/zoom/replay/reload paths pass. Edge lifecycle and editor containment remain blocking. |
| D-08 | FAIL | Runtime validation is not the same contract as the canonical JSON schema, and migration readiness can be reported before the backend echo is verified. |
| D-09 | PARTIAL | Standard Fib levels and direction control exist, but schema compatibility and directional/handle edge cases remain open. |
| D-10 | Provisionally accepted | Off/OHLC behavior and visible-candle-only input pass the ordinary path. Null time conversion must not fall back to the replay date. |
| D-11 | PARTIAL | Ten remounts and ordinary cleanup pass; active-drag tool switching and null/left-direction cases are untested and incorrect by inspection. |

## Independent verification

- Current checkout: `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; no branch/worktree/commit/push/merge action was performed by Reviewer.
- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 17 files/70 tests; lint/build PASS.
- Independent `./scripts/run-product-uat.sh`: PASS after rerunning with localhost permission. Result `test-results/product-uat/2026-07-18T01-37-13-462Z/results.json` records 174 passes, 0 failures, `blockingFailed: 0`, and no runtime, indicator-request, or provider error.
- Reviewer inspected the independent wide/Fibonacci/Text/all-tools/reload/zoom and 1280×800 artifacts. The drawings render, persist, and remain attached, but the 60px selected-drawing editor is clipped and unreadable.
- Production DB SHA-256 remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

The green result does not close the batch because current assertions do not cover the contradictory schema, transaction, geometry, and visible-editor states below.

## Findings

### B3-R01 — P1 — Canonical schema v1 and runtime validation are different contracts

`docs/decision-packs/sumi-drawing-document-v1.schema.json` permits any nonempty `paneId` and Fibonacci arrays with two or more arbitrary ratios. `drawingDomain.ts` accepts only `paneId: "price"` and exactly the seven ordered standard ratios. A document valid under the published schema can therefore be rejected by the product. This contradicts the ExecPlan statement that TypeScript is aligned “exactly” with the schema and risks quarantining or refusing valid v1 data.

Reviewer decision for closure: Batch 3 is not yet approved, pre-Batch-3 canonical production data supports Horizontal only, and the professional product contract is price-pane-only with the standard seven Fib levels. The hardening batch is authorized to tighten the canonical JSON schema v1 to those exact semantics, with an explicit compatibility note proving existing Horizontal v1 records remain unchanged. Text must be nonempty after trimming. If evidence reveals a previously supported nonstandard canonical Fib/pane document, stop for a versioning decision instead of silently rewriting it.

Required closure:

- make JSON schema, TypeScript union, validator, fixtures, repository and migration share one contract;
- add a contract test that fails when schema constants and runtime semantics drift;
- preserve current Horizontal v1 records byte-semantically;
- document why the pre-approval schema correction is safe and how malformed/development-only nonstandard data is handled.

### B3-R02 — P1 — Hydration/migration and rapid commands are not transaction-safe

When legacy backend data is migrated, the controller sets the workspace to `ready` before the asynchronous backend `PUT` is echoed and verified. That migration write is outside the serialized command queue. A user edit can race it. Invalid backend JSON is quarantined but hydration returns an empty ready document; the next edit can overwrite the malformed remote payload without a visible reconciliation decision. In addition, queued callbacks are built from render-time `document` snapshots. Two immediate edits can enqueue the same expected revision; the second conflict path can restore local storage correctly but set React state back to the stale `before` document.

The accepted limitation remains: the opaque endpoint has no cross-client atomic database CAS. That does not excuse races within one Sumi workspace.

Required closure:

- treat hydration/promotion/backend echo as one serialized state transition and publish `ready` only after verified equality;
- keep malformed/ambiguous remote state visibly blocked after quarantine; do not silently overwrite it on the next command;
- derive each queued mutation from the latest committed document/revision, not a stale render closure;
- prove two immediate valid commands serialize and both survive, while conflict/failure leaves UI, local canonical, backend mirror and history mutually consistent;
- retain the explicit no-cross-client-CAS limitation without claiming atomicity.

### B3-R03 — P1 — Provider cancellation and coordinate fallback violate the recorded contract

`setTool()` calls `clearTransaction(false)`. If a user switches tools during an active drag, the provider keeps the preview-mutated document instead of restoring the exact pre-drag drawing, while no semantic commit is persisted. The controller has already received `change-preview`, so the UI can display uncommitted geometry. Cursor and Escape happen to call the rollback path, but another tool button does not.

`anchorAt()` also uses `currentTime()` when official `coordinateToTime()` returns `null`. The Batch 3 prompt and ExecPlan require invalid/null coordinate conversion to be rejected. Replacing a failed time conversion with the replay date fabricates an anchor and can create misleading geometry in blank chart space.

Required closure:

- every tool switch, Escape, pointer cancellation, unmount and destroy must rollback an active drag/preview exactly without revision/history/persistence changes;
- reject a null official time or price conversion; do not substitute the replay date;
- add focused and browser tests for tool switch during drag, tool switch during two-anchor preview, null time/price, pointer capture loss and post-destroy inertness.

### B3-R04 — P1 — Ray/Rectangle/Text geometry is incomplete for a professional editor

`rayEndpoint()` always draws to the right edge. If the second anchor is left of the origin, the rendered segment from the origin to the right edge does not pass through the direction anchor. Current tests cover only a rightward second anchor. Enforce the recorded rightward Ray contract and reject/retain preview for an invalid second anchor or invalid edit.

Rectangle rendering and hit testing expose handles only for the two stored diagonal anchors. The other two visible corners have no handles, despite the prompt requiring corner editing. Text accepts up to 2000 characters and a textarea can contain newlines, but canvas rendering uses one `fillText()` call and hit testing assumes one line; rendered content and selection bounds diverge for multiline/long notes.

Required closure:

- define and enforce valid Ray direction for creation, pointer edit and form edit; test the left/equal-time cases;
- render four Rectangle corner handles and map each corner edit back to the same two-anchor domain without reordering/corrupting direction;
- make Text rendering and hit bounds use one bounded multiline layout, or explicitly constrain and validate a documented single-line product contract;
- prove actual canvas handle dragging for Trendline, Ray, all Rectangle corners and Fibonacci endpoints.

### B3-R05 — P1 — The visible drawing editor is technically present but practically unusable

The selection editor is hard-coded to 60px inside a 72px icon rail. In both independent 1440×1000 and 1280×800 artifacts, tool names wrap into fragments, date values are clipped, prices are not readable as complete values, and Text content is squeezed into a few characters per line. This fails the prompt requirement that selected type/ID and relevant fields be understandable and that compact evidence contain no clipped core drawing controls.

The domain stores line/fill/text style plus visibility/lock, but the editor exposes only anchors, Fib reverse, Text and delete. It does not provide the tool-aware style/visibility/lock controls explicitly authorized and required by the Batch 3 prompt.

Required closure:

- keep a compact icon rail, but move selected-drawing properties into a readable inspector/popover/panel with labelled, nonclipped controls at 1440×1000 and 1280×800;
- display tool name and stable identity clearly; validate edits before persistence and show actionable errors;
- expose the existing domain fields relevant to each tool: line color/width/style, fill color/opacity where applicable, text color/font size, visibility and lock;
- make each settings mutation one command with exact undo/redo/persistence/reload behavior;
- add browser geometry assertions for editor containment/readability, not only element counts.

### B3-R06 — P1 — UAT claims more than it exercises

The checks named `endpoint-edit` change number inputs in the toolbar; they do not drag endpoint handles on the chart. Provider-focused tests exercise a Horizontal drag and ordinary creation, but not body/handle interactions for every tool. There is no assertion for schema/runtime equivalence, null time fallback, tool-switch rollback during drag, left/equal Ray direction, four Rectangle corners, multiline Text bounds, migration readiness race, malformed-remote blocking, rapid queued edits, or selected-editor containment.

Required closure:

- retain all 174 existing assertion IDs and keep them blocking;
- add real pointer-handle gestures with exact semantic anchor/corner results;
- add the missing schema, persistence, lifecycle, geometry and visible-editor assertions listed in B3-R01–B3-R05;
- machine evidence must distinguish input-form edits from canvas-handle edits;
- manually review fresh 1440×1000 and 1280×800 screenshots with the inspector open for a two-anchor tool, Fibonacci and Text.

## Scope protection

Keep Lightweight Charts v5, the Sumi domain, official primitive provider, existing backend endpoint, IndicatorEngine, Batch 1 and Batch 2 architecture. Do not install a drawing dependency, change chart libraries, add a backend migration, weaken acceptance, redesign trade/journal workflows, or begin Batch 4.

Use `docs/dev-prompts/BATCH_3_REVIEW_HARDENING_PROMPT.md` for the bounded continuation. Reviewer approval is required before Batch 4 planning.

## Independent hardening re-inspection — 2026-07-18

Status: **RETURNED FOR A SECOND BOUNDED CLOSURE. Batch 3 remains open and Batch 4 remains unauthorized.**

The hardening materially closes the visible inspector, ordinary tool-switch/capture rollback, rightward-Ray happy path, four-corner rendering, multiline Text layout, serialized rapid commands, and real endpoint gestures. The Reviewer independently reproduced both the standalone UAT and full product gate at 216/216 with zero runtime/provider/request errors and inspected the required wide/compact artifacts. Those green results are accepted as happy-path evidence, but they do not close the exact B3-R01–B3-R06 contract because four P1 issues remain.

### Independent evidence

- Provenance remained `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2` still resolves to `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`. No branch/worktree/commit/push/merge/reset/clean/checkout/tag action was performed.
- `git diff --check`: PASS.
- Frontend direct gates: 18 files / 85 tests PASS; lint PASS; production build PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed / 1 skipped; frontend 18 files / 85 tests; lint/build PASS.
- Independent standalone UAT: `test-results/product-uat/2026-07-18T03-49-11-695Z/results.json` — 216 passed / 0 failed / `blockingFailed: 0`; zero runtime, provider, or indicator-request errors.
- Independent full product gate: `test-results/product-uat/2026-07-18T03-50-26-303Z/results.json` — the same 216/0/0 result; `./scripts/verify-product.sh` PASS.
- Baseline comparison against `2026-07-18T01-37-13-462Z`: all 174 IDs remain present exactly once, no prior pass changed, 42 IDs are additive, and the prefix-based blocking classifier still covers all Batch 1–3/drawing checks.
- Reviewer inspected the independent `13-rectangle-four-handles.png`, `14-multiline-text-selection-bounds.png`, `15-two-anchor-inspector-1440x1000.png`, and `07-compact-1280x800.png`. The external inspector is readable/scrollable and does not overlap the order controls; the compact artifact has the inspector open.
- Production DB SHA-256 after all runs remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- No new dependency, community drawing provider, chart switch, private Lightweight Charts API, backend migration, telemetry, acceptance removal, or Batch 4 implementation was found. The legacy `DrawingToolRegistry` is still instantiated by the chart facade but receives only the default empty compatibility array in Replay; it has no production input ownership. Remove or document this dormant renderer in the closure rather than allowing it to become a second authority.

### B3-R07 — P1 — B3-R01 is not closed: the published schema and runtime validator still accept different v1 documents

Concrete counterexamples remain:

- JSON Schema requires optional Fibonacci level `color` to have `minLength: 1`, while `validFibGeometry()` accepts every string including `""`.
- JSON Schema accepts any anchor time matching `YYYY-MM-DD`, including an impossible date such as `2026-02-30`; runtime `isDrawingDate()` rejects it.
- JSON Schema constrains a Ray to two anchors and `geometry.kind: ray` but has no rightward/later-anchor constraint; runtime rejects leftward/equal dates.

The claimed drift test only reads a few schema constants (`paneId`, ratios, item counts, Text min/max). It never validates the same positive/negative corpus through the published schema and runtime semantics, so it cannot detect these counterexamples. The browser check `batch3.hardening.runtime-canonical-v1-contract` only inspects already-valid runtime state and is not schema parity evidence.

Required closure:

- remove every expressible schema/runtime mismatch, including optional color emptiness and calendar-date semantics;
- explicitly define how cross-field/document invariants that standard JSON Schema cannot express (rightward Ray, duplicate IDs, contiguous order) are part of the canonical v1 contract rather than claiming literal validator equivalence;
- run one shared positive/negative corpus through an actual Draft 2020-12 schema validator and the runtime/domain semantic validator, with explicit expected outcomes for every supplemental invariant;
- preserve the existing valid Horizontal v1 byte semantics and stop if any supported nonstandard canonical record is discovered.

### B3-R08 — P1 — B3-R02 is not closed for an indeterminate or mismatched backend write outcome

`persistMutation()` writes the new local revision, performs the backend `PUT`, then restores the prior local/UI document immediately if the request throws or the echoed payload differs. It does not re-read the backend after that outcome. If the server committed before a connection failure, or committed/returned a different payload, the actual backend mirror may now differ from the restored UI/local/history while the controller reports only `error`/`conflict`.

The current failure mock throws before assigning `backendRaw`, so it proves only a pre-commit failure. It does not cover commit-then-error, echo mismatch after mutation, or reconciliation of the three possible remote outcomes: prior value, intended value, or third-party/divergent value.

Required closure:

- after any indeterminate PUT failure or echo mismatch, reconcile with a serialized GET before declaring the transaction outcome;
- if backend equals the intended canonical document, finish the commit exactly once; if it equals the prior mirror, roll back exactly; if it is any third value or remains unavailable, quarantine/block with honest indeterminate/conflict state and preserve both recovery copies;
- apply the same policy to migration and ordinary/undo/redo commands without claiming cross-client atomic CAS;
- add focused tests for commit-then-throw, mismatched echo that did mutate remote, unavailable reconciliation, and migration equivalents, asserting UI/local/backend/history/status equality or explicit blocked ambiguity.

### B3-R09 — P1 — B3-R03/R04 remain unsafe when one body-drag coordinate conversion fails

Endpoint/corner conversion rejects a null official coordinate. Body movement instead maps anchors independently and silently substitutes the original anchor for any failed conversion. If one anchor converts and the other does not, the provider can preview and commit a partially moved/deformed Trendline, Ray, Rectangle, or Fibonacci drawing. That violates the exact null-coordinate rejection contract and the requirement that body movement preserve geometry.

Required closure:

- make a body drag atomic: compute every translated anchor first and reject the entire preview/commit when any time/price/date conversion is null or invalid;
- prove no document/revision/history/persistence mutation and clean capture/scroll state after the rejected gesture;
- add focused tests for one-of-two anchor time failure and price failure for every two-anchor body, plus successful equal logical-time/price deltas for all bodies.

### B3-R10 — P1 — B3-R06 still overstates exact canvas semantics

The additive real-pointer checks are useful, but `dragCanvasHandle()` passes whenever the selected part is reported, the revision increments once, and *any* anchor changes. It does not assert that only the intended endpoint/fields changed. All four Rectangle-corner gestures use `dx = 0`, so they exercise only price changes and never prove each mixed corner's time-source mapping. The focused mixed-corner test likewise keeps x/time unchanged. Baseline body-move checks assert only that anchors differ, not equal logical-time/price deltas or geometry preservation. There are no browser IDs for null body conversion or the indeterminate backend outcomes above.

Required closure:

- keep all 216 current IDs, names, pass values, and blocking semantics;
- strengthen/add pointer assertions for both x and y on every Rectangle corner and exact unchanged/changed fields for Trendline/Ray/Fib endpoints and Text anchor;
- assert equal logical-index and price deltas for each body and no commit for partial/null conversion;
- add observable persistence checks for commit-then-error/echo-mismatch reconciliation and a real schema/runtime negative corpus check; do not treat already-valid hidden state as contract proof;
- retain fresh 1440×1000 and 1280×800 screenshots and zero runtime/provider/request errors.

### Accepted hardening and limitations

- B3-R05 is closed: the selected inspector is outside the 72px tool rail, supports the authorized domain fields with Apply/Cancel/validation/keyboard isolation, remains contained at both required viewports, and leaves the chart usable under the existing fixed-responsive policy.
- Ordinary B3-R02 rapid-command serialization is accepted: two immediate valid commands derive from the committed ref and survive as distinct revisions. Malformed remote input remains quarantined/blocked. Only the post-PUT indeterminate reconciliation gap above remains.
- Ordinary B3-R03 rollback paths are accepted for tool switch, Escape/Cursor, pointercancel, lost capture, unmount, and destroy. The provider no longer fabricates the replay date. Only partial body conversion remains open.
- Rightward Ray creation/edit validation, four visible Rectangle handles, shared multiline Text render/hit bounds, and real pointer gestures are accepted as implemented direction; exact field/delta and null-edge proof remains open.
- The backend endpoint still lacks atomic cross-client database CAS. This is an accepted, explicit limitation only after single-client indeterminate outcomes are reconciled honestly; no cross-client atomicity may be claimed.

Use `docs/dev-prompts/BATCH_3_SECOND_REVIEW_CLOSURE_PROMPT.md` for the next bounded DEV continuation. Do not start Batch 4.

## Final Reviewer closure — 2026-07-18

Status: **APPROVED AND CLOSED. Batch 4 is authorized only through `docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md`.**

The Reviewer independently inspected the second closure implementation, focused tests, complete gates, machine-readable UAT, assertion preservation, and fresh browser artifacts. B3-R07 through B3-R10 are closed, and D-01 through D-11 are accepted for the Drawing MVP scope.

### Closure decision

- **B3-R07 closed:** the Draft 2020-12 schema and runtime semantics use one 20-case positive/negative corpus. AJV 8 validates the structural contract, while explicitly named supplemental semantics cover strictly rightward Rays, unique IDs, contiguous order, and workspace identity. Empty optional colors and impossible dates are rejected consistently; existing valid Horizontal v1 serialization is preserved.
- **B3-R08 closed:** migration, ordinary changes, undo, and redo reconcile a dispatched PUT failure or mismatched echo through a serialized GET. Intended remote state commits exactly once, prior state rolls back exactly, and divergent/unavailable state preserves prior/intended/observed evidence, reports `indeterminate`, and blocks later writes until reload/reconciliation.
- **B3-R09 closed:** every multi-anchor body move converts and validates all anchors before publishing a preview. A failed time or price conversion rejects the whole gesture, restores the exact document, releases capture, restores scrolling, and emits no commit. Successful two-anchor body moves preserve equal logical-index and price deltas.
- **B3-R10 closed:** real browser pointer checks use nonzero x/y motion, verify the exact changed and unchanged anchor fields for endpoints and all four Rectangle corners, retain rightward Ray semantics, verify equal body deltas, and exercise the schema corpus and reconciliation branches with eight additive blocking IDs.

### Independent final evidence

- Provenance remained `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; annotated tag `v2.0.0-rc2^{}` remained `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`. No branch/worktree/stage/commit/push/merge/reset/clean/checkout/tag operation was performed.
- Focused second-closure tests: 3 files / 57 tests passed. Complete frontend: 18 files / 107 tests passed; lint and production build passed.
- `./scripts/verify-v2.sh`: passed — backend 75 passed / 1 skipped; frontend 107 tests; lint/build passed.
- Independent standalone UAT: `test-results/product-uat/2026-07-18T06-23-17-806Z/results.json` — 224 passed / 0 failed / `blockingFailed: 0`.
- Independent full product gate: `test-results/product-uat/2026-07-18T06-24-40-732Z/results.json` — 224 passed / 0 failed / `blockingFailed: 0`; `runtimeErrors`, `providerErrors`, and `indicatorRequestFailures` are all empty.
- Comparison with the accepted 216-ID baseline `2026-07-18T03-50-26-303Z`: missing IDs `[]`, duplicates `[]`, changed pass values `[]`, and exactly eight additive blocking `batch3.second-closure.*` IDs.
- Reviewer inspected the independent 1440×1000 Rectangle, multiline Text, and two-anchor inspector screenshots and the 1280×800 compact screenshot. Four Rectangle handles, bounded multiline Text, labelled complete anchors, and inspector containment are visible; the chart and replay/trade controls remain available.
- `git diff --check` passed. Production DB SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d` after every gate.

### Accepted limitations and scope boundary

- The opaque backend drawings endpoint has no database compare-and-swap across concurrent clients. Batch 3 guarantees serialized single-client preflight/echo/reconciliation and safely blocks ambiguous remote outcomes; it does not claim cross-client atomicity.
- Dev-only `ajv@8.17.1` is accepted for genuine Draft 2020-12 contract verification. It is not imported by production code or added to the production bundle; the resolved licenses recorded in the ExecPlan are MIT/BSD-3-Clause.
- This approval closes the **Batch 3 Drawing MVP only**. It does not declare Sumi a complete professional product, “TradingView-like,” release-ready, or finished under all V3 acceptance criteria.
- Batch 4 may begin only in a separate DEV session using the standalone Batch 4 prompt. Batch 5 remains unauthorized.
