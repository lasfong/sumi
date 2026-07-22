# DEV prompt — Batch 3 second Reviewer closure

Work directly in `/Users/mizuhara/workspace/sumi` on the current dirty checkout. This is a new DEV session with no assumed conversation history. Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, checkout, retag, discard, overwrite, stage, or reorganize existing user/Reviewer/DEV changes.

This prompt authorizes only B3-R07 through B3-R10 in `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`. Batch 3 remains unapproved. Batch 4 and Batch 5 are not authorized. End at the Reviewer gate.

## Product purpose and claims

Sumi is a local-first manual replay and backtesting workstation for serious personal technical-analysis practice on Vietnam market data. Do not call it “TradingView-like”, professional-product-complete, release-ready, or V3-complete. This closure concerns only the Drawing MVP contract.

## Read completely before editing

Read in this order:

1. `AGENTS.md`
2. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
5. `docs/DEVELOPMENT_OPERATING_MODEL.md`
6. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
7. `PLANS.md`
8. `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`
9. `docs/decision-packs/sumi-drawing-document-v1.schema.json`
10. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
11. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
12. all of `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`, including the independent hardening re-inspection
13. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`
14. `docs/exec-plans/BATCH_2_PROFESSIONAL_INDICATOR_MANAGER.md`
15. `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`
16. `docs/dev-prompts/BATCH_3_REVIEW_HARDENING_PROMPT.md`
17. the complete drawing domain/schema/repository/history/controller/provider/geometry/inspector/UAT implementation and focused tests.

Batch 1 Replay foundation and Batch 2 Indicator Manager are approved blocking baselines. The accepted portions of Batch 3 hardening—external inspector, ordinary rollback, ordinary rapid serialization, rightward-Ray interaction, four handles, multiline Text, and 216 current UAT assertions—are also blocking regressions.

## Before product code

Append a “Second Reviewer closure — B3-R07–B3-R10” section to `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`. Record:

- exact scope and affected modules;
- canonical-schema strategy and supplemental semantic invariants;
- indeterminate-write reconciliation state machine;
- atomic body-drag conversion policy;
- exact UAT IDs to retain/add/strengthen;
- rollback, compatibility, stop conditions, and all verification commands.

Record current branch/HEAD/tag, dirty-tree inventory, and the production DB SHA-256 before editing. Preserve all existing files and evidence.

## Non-negotiable invariants

- Never expose future candles beyond `current_index`; provider/magnet receives visible candles only.
- Backend `IndicatorEngine` remains authoritative.
- Automated tests/UAT use a temporary database and never mutate `backend/sumi.db`.
- Keep Lightweight Charts v5 and the Sumi official-primitives provider. Do not add a dependency, community provider, fork, private API, or chart switch.
- Do not change the backend database schema or drawing endpoint contract.
- No telemetry or external transmission of market/trading data.
- Keep one Sumi drawing domain/controller authority, one provider primitive, and one listener set. The dormant legacy renderer must not become a mutable production authority; remove its Replay instantiation if safe or document and test its strict empty/read-only compatibility role.
- Preserve all 216 current UAT IDs, names, pass values, and blocking semantics. Do not weaken, rename, delete, skip, hard-code, or hide an assertion.

## B3-R07 — one honest canonical v1 contract

Current counterexamples to close:

- schema rejects Fibonacci level `color: ""`; runtime accepts it;
- schema accepts a pattern-matching impossible calendar date; runtime rejects it;
- schema has no cross-anchor rightward-Ray assertion; runtime rejects left/equal dates.

Required outcome:

- align every constraint expressible in Draft 2020-12, including nonempty optional colors and real calendar-date semantics;
- document canonical v1 as the published structural schema plus explicitly named Sumi semantic invariants where standard JSON Schema cannot express cross-item/document relationships: strictly rightward Ray, unique drawing IDs, contiguous order, and identity checks;
- do not claim that checking four constants proves validator equivalence;
- use one shared positive/negative fixture corpus. Run it through an actual Draft 2020-12 schema validator in tests and through runtime/domain validation, then assert expected structural and supplemental-semantic outcomes separately;
- include malformed/future/unknown/tool-geometry/anchor-count/pane/Fib/Text/style/date/Ray/duplicate/order cases;
- keep valid Horizontal v1 serialization and meaning unchanged. If a supported nonstandard canonical v1 record is found, stop with its exact payload shape for Reviewer versioning decision.

Do not add a production runtime dependency solely for validation. A test-only validator already present transitively is not a safe implicit contract: if needed, record an explicit dev-only dependency and license/lockfile impact, or implement a bounded test validator without changing runtime dependency posture. Stop if this requires a new production dependency or schema v2.

## B3-R08 — reconcile indeterminate backend writes

The opaque backend has no atomic database CAS; do not claim otherwise. Within one client identity queue, make migration and ordinary/undo/redo outcomes honest:

1. Preflight the expected remote mirror.
2. Prepare the next local CAS value.
3. PUT the intended canonical document.
4. On normal exact echo, commit once.
5. On request failure after dispatch or mismatched echo, perform a serialized reconciliation GET before deciding.
6. If remote equals intended, complete UI/local/history exactly once.
7. If remote equals prior, restore UI/local/history exactly.
8. If remote is a third value or cannot be read, preserve prior and intended recovery evidence, mark an explicit blocked/indeterminate conflict, and pause writes until reload/reconciliation. Never silently overwrite it.

Apply the same decision table to legacy migration. Do not fabricate atomicity with a mock that throws before remote mutation.

Required focused tests:

- PUT commits intended remote then throws;
- PUT mutates remote and returns a mismatched echo;
- reconciliation reports prior remote;
- reconciliation reports intended remote;
- reconciliation reports third/divergent remote;
- reconciliation GET is unavailable;
- each case for ordinary commit, undo/redo history acceptance where relevant, and migration;
- exact UI/local/backend/history/status/backup equality or explicit blocked ambiguity after every outcome.

## B3-R09 — atomic body-drag coordinate conversion

For Trendline, Ray, Rectangle, Fibonacci, and any other multi-anchor body move:

- project the original drawing;
- calculate the logical-time and price translation for every anchor;
- reject the entire preview when any official `coordinateToTime`/`coordinateToPrice` result is null, invalid, nonpositive, or not a valid session date;
- never retain one original anchor while moving another;
- successful body movement must preserve equal logical-index and price deltas, tool direction, Rectangle geometry, and Fib semantics;
- rejected movement produces no preview residue, command, revision, history, local write, backend write, or dirty capture/scroll state;
- pointerup after a rejected preview must not commit.

Add focused provider tests for one-of-two time failure and one-of-two price failure for each two-anchor body, plus successful exact-delta tests. Retain endpoint/corner null rejection and all ordinary cancellation behavior.

## B3-R10 — exact real-pointer/UAT proof

Retain all 216 current assertions. Add or strengthen blocking browser checks so their boolean conditions assert the contract rather than merely record it in evidence:

- Trendline/Ray/Fib endpoint 0 changes only anchor 0; endpoint 1 changes only anchor 1; Ray remains strictly rightward.
- Text anchor changes its sole anchor only.
- Drag every Rectangle corner with both nonzero `dx` and `dy`. Assert the exact time-source and price-source fields that change and the exact fields that remain unchanged for corner 0/1/2/3 while canonical storage remains two anchors.
- For every body, assert equal logical candle-index delta and equal price delta across anchors; do not accept “anchors differ” as proof.
- Add a browser-visible/provider test seam for a partial/null body conversion and assert no revision/history/persistence change plus clean interaction state. Do not patch private chart APIs.
- Add browser/machine checks for commit-then-error, echo mismatch, and blocked third/unknown remote outcomes using controlled API routing plus exact UI/local/backend/history/status evidence.
- Replace the existing valid-state-only “runtime canonical contract” proof with machine evidence from the shared negative/positive schema/runtime corpus. Hidden declarations alone are insufficient user-visible proof.
- Keep waits observational: every wait must be followed by exact state/outcome assertions and must not swallow a failed persistence/render expectation.
- Continue capturing runtime errors, provider errors, failed API outcomes, and indicator request failures.

Retain and manually inspect fresh evidence at:

- 1440×1000: all tools/handles, Rectangle four-corner x+y edits, multiline Text bounds, two-anchor inspector, pan/zoom/replay/reload;
- 1280×800: inspector open, chart usable, no clipped core drawing controls, no overlap with replay/trade controls.

## Accepted scope that must remain green

- D-01 tool inventory, labels/tooltips and obvious active mode.
- Selection, delete, confirmed Clear All, undo/redo, magnet Off/OHLC, visible-candle-only input.
- External inspector fields, Apply/Cancel/validation, visibility/lock, styles, Text/Fib semantics, keyboard isolation, reload.
- Ordinary tool switch, Escape/Cursor, pointercancel, capture loss, unmount and idempotent destroy rollback.
- Rightward Ray creation/edit validation, four visible Rectangle handles, bounded multiline Text render/hit/select.
- Session/symbol isolation, legacy backup/quarantine/idempotence and ordinary rapid two-command serialization.
- Batch 1 and Batch 2 browser assertion baselines and zero runtime/page/console errors.

## Verification

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

Use only isolated temporary backend/UAT databases. Retain the result directory, screenshots, backend/frontend logs, and machine-readable output. Compare the final IDs with the 216-ID Reviewer baseline `test-results/product-uat/2026-07-18T03-50-26-303Z/results.json`; report missing, duplicate, renamed, changed-pass, or changed-blocking IDs explicitly.

## Stop conditions

Stop and return evidence to Reviewer rather than guessing if closure requires:

- schema v2 or reinterpretation of a supported nonstandard v1 document;
- backend contract/database migration or a cross-client atomicity claim;
- dependency/provider/fork/private chart API/chart-library change;
- weakening/removing/renaming an assertion;
- broad Replay/trade/journal/Indicator redesign;
- Batch 4 or Batch 5 work.

## Final DEV handoff

Update the Batch 3 ExecPlan without rewriting prior DEV/Reviewer history. Report B3-R07–B3-R10 individually, D-01–D-11 status, exact files, tests/counts, UAT artifact path, screenshot review, runtime/provider/request errors, assertion comparison, deviations, accepted cross-client-CAS limitation, and DB hash before/after.

Do not self-approve Batch 3. Do not create a Batch 4 prompt. End with exactly:

`BATCH 3 SECOND CLOSURE COMPLETE — STOPPED AT REVIEWER GATE`
