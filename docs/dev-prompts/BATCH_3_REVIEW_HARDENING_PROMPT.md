# DEV prompt — Batch 3 Reviewer hardening

Continue in the existing Sumi checkout and dirty working tree. Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, checkout, retag, discard, or overwrite existing Reviewer/user changes.

This prompt authorizes only the bounded closure of B3-R01 through B3-R06 in `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`. Batch 3 is not approved. Batch 4 is not authorized.

## Read before editing

Read completely:

1. `AGENTS.md`
2. `PLANS.md`
3. all canonical V3 sources listed in `AGENTS.md`
4. `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`
5. `docs/decision-packs/sumi-drawing-document-v1.schema.json`
6. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
7. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
8. `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`
9. `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`
10. the complete drawing domain/repository/history/controller/provider/geometry/magnet/toolbar/workspace/UAT implementation and tests.

Before product edits, append a Reviewer-hardening section to the Batch 3 ExecPlan. Record B3-R01–B3-R06, exact modules, compatibility decision, interaction changes, failure/rollback policy, additive UAT IDs, exact commands and stop conditions.

Record the production DB SHA-256 before work. Preserve all existing Batch 1/2/3 accepted behavior and every current UAT assertion ID.

## Required closure

### B3-R01 — one canonical schema-v1 contract

Make `sumi-drawing-document-v1.schema.json`, TypeScript types, runtime validation, fixtures, migration and repository agree.

Reviewer authorizes a pre-approval v1 schema correction to the already intended product semantics:

- `paneId` is exactly `price`;
- Fibonacci has exactly the ordered ratios `0, .236, .382, .5, .618, .786, 1`, with visibility and optional color;
- Text canonical content is nonempty after trimming and no longer than the recorded maximum;
- geometry kind matches tool and anchor counts exactly;
- existing valid Horizontal v1 documents keep the same meaning and round-trip unchanged.

Document that Batch 3 was not approved before this correction and no approved non-Horizontal v1 product record existed. If you discover evidence of previously supported canonical nonstandard Fib/pane data, stop at Reviewer gate with the payload shape and do not silently reinterpret it.

Add a contract test that reads the JSON schema and detects drift from runtime constants/semantics. Do not add a new runtime dependency merely for this test.

### B3-R02 — serialized hydration, migration and commands

Refactor narrowly so there is one transaction authority per session/symbol:

- hydration/promotion remains `loading` until backend canonical echo is verified;
- migration PUT participates in the same serialized queue as later commands;
- failed migration retains local backup/recovery, reports `error`, and pauses semantic commits;
- malformed/ambiguous remote state is quarantined and visibly blocked; a later edit must not overwrite it implicitly;
- every queued mutation is derived from the latest committed document/revision at execution time;
- two immediate valid edits serialize to two durable revisions without conflict or lost state;
- conflict/failure leaves React document, canonical local value, backend mirror and history consistent;
- same-identity divergence remains blocked and the no-cross-client-database-CAS limitation stays explicit.

Use the current opaque backend endpoint and temporary DB. Do not change the backend contract or add a migration.

### B3-R03 — exact cancellation and official coordinate failure

- Tool switching during an active drag must rollback the exact pre-drag anchors, selection, local value, revision and history before activating the new tool.
- Tool switching during a two-anchor preview must remove the preview without a commit.
- Escape, Cursor, native `pointercancel`, capture loss, unmount and `destroy()` must use consistent rollback/cleanup semantics.
- `coordinateToTime() === null`, `coordinateToPrice() === null`, invalid dates/prices and unavailable pane bounds must reject the gesture. Remove the `currentTime` fallback for conversion failure.
- No rejected gesture may create a drawing, mutate a document or leave scrolling/capture/listeners dirty.

### B3-R04 — complete geometry and real handles

- Enforce the recorded rightward Ray contract: the direction anchor must be later/right of the origin for create, pointer edit and inspector edit. An invalid second anchor/edit must not fabricate a ray or corrupt the prior valid drawing.
- Render and hit-test all four Rectangle corner handles. Keep the two-anchor canonical domain; map mixed corners deterministically back to time/price fields without reordering domain anchors.
- Body moves preserve geometry; endpoint/corner edits modify only the intended semantic fields.
- Make Text render/hit-test/select consistently for its accepted content. Prefer a bounded multiline layout with explicit newline handling and the same measured bounds for rendering and hit testing. If choosing a single-line contract, document and validate it consistently in schema/UI/migration; do not silently discard newlines.
- Maintain one primitive and one listener set.

### B3-R05 — professional, readable drawing inspector

Do not place the property editor inside the 60px icon rail. Keep the rail compact and create a Sumi-owned responsive inspector/popover/panel for the selected drawing.

At both 1440×1000 and 1280×800:

- tool name, stable ID (a readable short form plus full accessible/title value), anchors and validation states are understandable and not clipped;
- inputs have visible labels and complete values;
- core chart interaction remains usable and the inspector does not cover essential replay/trade controls;
- keyboard focus, Escape and Delete do not accidentally invoke chart commands while editing a field.

Expose and persist the existing domain properties relevant to the tool:

- line color, line width and line style;
- fill color/opacity for Rectangle/Fibonacci where applicable;
- text color/font size for Text;
- visibility and lock;
- anchors, Fibonacci direction and Text content.

Each accepted inspector change is one semantic history command and one serialized persistence operation. Cancel/invalid input changes nothing. Undo/redo and reload restore exact values.

This is closure of the already authorized Batch 3 editor contract, not authorization for a general design-system rewrite.

### B3-R06 — additive proof

Keep all 174 current assertion IDs, names and blocking status. Add focused tests and browser checks that fail before the hardening and pass only with real behavior:

- JSON-schema/runtime contract equivalence and existing Horizontal compatibility;
- migration remains non-ready until verified echo; migration failure; malformed remote blocking; backup/quarantine; two immediate edits; same-revision conflict; local/backend/UI/history equality after every outcome;
- tool switch mid-drag rollback; tool switch mid-preview; null time/price; pointercancel/capture loss; idempotent destroy;
- Ray rightward valid creation plus left/equal invalid creation and edit;
- actual pointer dragging of both Trendline endpoints, both Ray endpoints, all four Rectangle corners, both Fib endpoints, Text anchor and bodies of all tools;
- exact one-revision/one-history behavior for each gesture;
- multiline/long Text render and hit-bound behavior under the chosen contract;
- inspector style/visibility/lock edits, cancel, invalid values, undo/redo, persistence and reload;
- inspector containment, complete visible values and keyboard isolation at 1440×1000 and 1280×800;
- ten remounts followed by one gesture still produces one command/revision/listener response.

Do not count an inspector number-field edit as proof of canvas endpoint/corner dragging. Machine evidence must record the gesture part and exact before/after anchors.

Capture and manually review fresh artifacts with:

- two-anchor inspector open at 1440×1000;
- all tools and selected handles;
- four Rectangle handles;
- Fibonacci labels/direction and inspector;
- multiline Text plus its selection bounds;
- post-reload and pan/zoom/replay state;
- inspector open at 1280×800 with no clipped core controls.

## Required verification

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

Use only an isolated temporary database for automated/UAT writes. Retain machine-readable results, screenshots and runtime logs. Compare current assertion IDs with the new run and report missing/renamed/weakened IDs explicitly.

## Stop conditions

Stop and ask Reviewer with evidence rather than guessing if closure requires:

- schema version 2 or reinterpretation of a previously supported nonstandard canonical v1 record;
- backend contract/database migration or a cross-client atomicity claim;
- another drawing/chart dependency, fork or private Lightweight Charts API;
- weakening/removing an assertion;
- broad Replay/trade/journal redesign;
- Batch 4 work.

## Final handoff

Update the Batch 3 ExecPlan with decisions, exact verification output, artifacts, deviations and self-review. Report B3-R01–B3-R06 individually, all D-01–D-11 statuses, every remaining failure ID, runtime/provider/request errors and production DB before/after hash.

End with:

`BATCH 3 HARDENING COMPLETE — STOPPED AT REVIEWER GATE`

Do not claim Batch 3, Sumi, or the drawing system is approved/product-complete. Do not start Batch 4.
