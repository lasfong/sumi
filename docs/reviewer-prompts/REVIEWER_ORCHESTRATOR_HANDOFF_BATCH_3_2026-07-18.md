# Reviewer/Orchestrator handoff — Batch 3 hardening gate

You are the new **Reviewer/Orchestrator session** for Sumi. Work directly in `/Users/mizuhara/workspace/sumi` on the current checkout. This is a review and orchestration task, not a DEV implementation task.

Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, checkout, retag, discard, or overwrite existing user/Reviewer/DEV changes. Do not edit product code. Reviewer-owned documentation, review reports, roadmap/index updates, and DEV prompt files may be updated after evidence-backed judgement.

The previous Reviewer session was intentionally retired because its context had become long after Batch 0 through Batch 3. Do not assume its conclusions are correct merely because they are recorded below. Reconstruct the current truth from the repository, diff, tests, browser behavior, and artifacts.

## Product and operating context

Sumi is a local-first manual replay and backtesting workstation for serious personal technical-analysis practice on Vietnam market data. It is not a TradingView clone. Do not use “TradingView-like” or declare the product professional/product-complete unless the applicable V3 acceptance contract and real browser workflow support that language.

Non-negotiable invariants:

- future candles must never be returned or exposed beyond `current_index`;
- backend `IndicatorEngine` remains authoritative for indicator computation;
- `v2.0.0-rc2` must not be moved, retagged, or rewritten;
- automated tests and UAT must use a temporary database and must not mutate `backend/sumi.db`;
- no telemetry or external transmission of market/trading data;
- no drawing/chart dependency without an approved spike/license/provider decision;
- browser evidence is required for user-facing chart work;
- one bounded batch at a time; no acceptance weakening;
- current checkout/branch is used unless the user explicitly requests isolation.

Read `AGENTS.md` completely, then read the canonical sources in its required order. Also read:

1. `PLANS.md`
2. `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`
3. `docs/decision-packs/sumi-drawing-document-v1.schema.json`
4. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
5. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
6. `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`
7. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`
8. `docs/exec-plans/BATCH_2_PROFESSIONAL_INDICATOR_MANAGER.md`
9. `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`
10. `docs/dev-prompts/BATCH_3_REVIEW_HARDENING_PROMPT.md`

## Provenance known at handoff

Verify rather than trust this snapshot:

- repository: `/Users/mizuhara/workspace/sumi`;
- current branch: `master`;
- current HEAD: `108aa5dc0e26994607836e2b3b33f482e3791b4e` (`merge: post-rc2 hardening`);
- the checkout intentionally contains uncommitted Batch 0–3 implementation, governance, harness and evidence files;
- no branch/worktree/commit/push/merge was used for these batches;
- production DB expected SHA-256: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`;
- Lightweight Charts v5 and the Sumi official-primitives drawing provider remain the approved chart/drawing direction;
- Deepentropy and Difurious were rejected for production in Batch 0;
- Batch 1 Replay foundation and Batch 2 Indicator Manager are Reviewer-approved and must remain green;
- Batch 4 has not been authorized.

Start with:

```bash
git status --porcelain
git branch --show-current
git rev-parse HEAD
git log --oneline -10
git tag --points-at HEAD
git rev-list -n 1 v2.0.0-rc2
git diff --check
shasum -a 256 backend/sumi.db
```

Do not treat the large dirty tree as permission to delete, reset, stage, or reorganize unrelated files.

## Completed batch history

### Batch 0

- Deterministic product harness and V3 governance were established.
- Community drawing candidates were spiked and rejected.
- Decision: keep Lightweight Charts v5 and build `SumiPrimitiveDrawingProvider` using official primitives behind a Sumi-owned provider/domain boundary.

### Batch 1 — approved

- Replay route/workspace/controller boundaries were established.
- Horizontal Line vertical slice, versioned local repository, provider lifecycle, undo/redo and official price-pane input boundaries were hardened.
- Identity keys are session/symbol scoped; pointer cancellation and revision conflict semantics were independently verified.

### Batch 2 — approved

- Professional Indicator Manager for EMA/RSI/MACD/CCI/Volume was implemented.
- Backend calculation remains authoritative.
- Stable instances, settings, hide/show/remove, persistence, pane chrome/reference values and deterministic fixed-responsive layout were independently verified.
- Final approved independent evidence before Batch 3 recorded 79 passes and seven drawing gaps.

### Batch 3 — initial implementation returned

The initial Batch 3 implementation produced a real Sumi-owned drawing system and 174/174 UAT, but the previous Reviewer returned six P1 findings:

- **B3-R01:** schema-v1 JSON contract and runtime validator differed;
- **B3-R02:** hydration/migration and rapid commands were not one safe serialized transaction;
- **B3-R03:** tool-switch rollback and null official-coordinate handling violated the interaction contract;
- **B3-R04:** Ray direction, four Rectangle corners and multiline Text geometry were incomplete;
- **B3-R05:** selected-drawing editor was clipped inside a 60px rail and omitted authorized semantic settings;
- **B3-R06:** UAT called form input edits endpoint edits and did not prove real chart-handle interactions/edge cases.

The authoritative finding text and closure requirements are in `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`. Review against those exact requirements, not this abbreviated list.

## Current DEV handoff to review

DEV reports **“BATCH 3 HARDENING COMPLETE — STOPPED AT REVIEWER GATE”** and does not self-approve the batch.

Reported closure:

- B3-R01–B3-R06: all PASS for Reviewer inspection;
- D-01–D-11: all PASS for Reviewer inspection;
- schema v1/TypeScript/runtime aligned while preserving Horizontal v1 semantics;
- a serialized authority now owns hydration, migration and commands;
- drag/preview/null-coordinate/capture-loss/destroy cleanup hardened;
- rightward Ray, four Rectangle corners and shared multiline Text render/hit bounds implemented;
- responsive inspector moved outside the icon rail with Apply/Cancel, validation, keyboard isolation and persistence;
- real pointer UAT added without deleting or renaming baseline assertions.

Reported verification:

- backend: 75 passed / 1 skipped;
- focused drawing: 5 files / 44 tests;
- full frontend: 18 files / 85 tests;
- lint/build, `verify-v2.sh`, `verify-product.sh`, `git diff --check`: PASS;
- browser UAT: 216 passed / 0 failed / `blockingFailed: 0`;
- baseline 174 assertion IDs retained, 42 additive IDs, no duplicates;
- zero runtime/provider/indicator-request errors;
- production DB unchanged at the expected hash;
- no backend contract/schema migration/dependency/chart-base change;
- known limitation retained: backend opaque endpoint has no atomic cross-client database CAS; only single-client serialized/preflight/echo/conflict protection is claimed.

Canonical DEV evidence:

- `docs/exec-plans/BATCH_3_PROFESSIONAL_DRAWING_MVP.md`
- `test-results/product-uat/2026-07-18T03-15-14-250Z/results.json`
- `test-results/product-uat/2026-07-18T03-15-14-250Z/13-rectangle-four-handles.png`
- `test-results/product-uat/2026-07-18T03-15-14-250Z/14-multiline-text-selection-bounds.png`
- `test-results/product-uat/2026-07-18T03-15-14-250Z/15-two-anchor-inspector-1440x1000.png`
- `test-results/product-uat/2026-07-18T03-15-14-250Z/07-compact-1280x800.png`

## Your task — independent Batch 3 hardening review

Do not modify product code. Do not simply rerun tests and approve. Inspect the actual implementation and user-visible UI.

### 1. Inventory and diff audit

- Record provenance, dirty-tree inventory and production DB baseline.
- Inspect changed/untracked drawing, chart, replay, schema, UAT and test files.
- Confirm no new community dependency, chart switch, private Lightweight Charts API, backend migration, acceptance weakening or Batch 4 code.
- Confirm old legacy production renderer is not a second mutable authority.

### 2. B3-R01 schema contract

Inspect the canonical JSON schema, TypeScript discriminated union, runtime validator, fixtures and schema contract test.

Verify:

- price pane is the only accepted pane;
- exact standard Fib levels/order/direction agree everywhere;
- Text trimming/nonblank/maximum and multiline semantics agree;
- tool/geometry/anchor counts and rightward Ray constraints agree;
- existing Horizontal v1 fixtures round-trip without semantic change;
- future/unknown/malformed/nonstandard documents are handled consistently;
- DEV did not silently reinterpret evidence of a previously supported canonical nonstandard v1 document.

### 3. B3-R02 persistence and migration

Inspect `DrawingRepository`, controller queue/transaction code, API adapter and focused tests.

Verify:

- hydration and migration remain loading until verified backend echo;
- migration write and later commands share one identity queue;
- malformed/ambiguous remote state is quarantined and conflict-blocked rather than overwritten by the next edit;
- two immediate valid commands derive from the latest committed document and survive as distinct revisions;
- ordinary/undo/redo conflict and backend/local failure leave UI, local canonical value, backend mirror and history consistent;
- session/symbol isolation, legacy backup, idempotence and cursor exclusion remain intact;
- no cross-client atomicity is claimed or simulated by a weak test.

### 4. B3-R03 provider lifecycle

Inspect the official primitive provider and real browser paths.

Verify exact rollback/no revision/no persistence for:

- tool switch mid-drag;
- tool switch mid-two-anchor preview;
- Cursor and Escape;
- native pointer cancellation and capture loss;
- unmount and idempotent destroy;
- null time/price conversion and unavailable price-pane bounds.

Confirm no fallback fabricates the current replay date when official coordinate conversion fails. Confirm listener/primitive ownership remains one provider primitive and one listener set rather than one per drawing.

### 5. B3-R04 geometry

Verify with code, focused tests and browser evidence:

- Ray can only commit with a strictly rightward/later direction anchor and invalid create/edit does not corrupt the prior document;
- Trendline/Ray/Fib endpoints are genuinely dragged on canvas;
- Rectangle visibly exposes four corners and each corner updates the correct time/price fields while the canonical domain remains two anchors;
- body moves preserve each tool's geometry;
- multiline Text renderer, ellipsis/bounds and hit testing use the same layout and keep canonical text intact;
- selection handles/bounds remain aligned after pan, zoom, replay, resize and reload.

### 6. B3-R05 product UX

Run/inspect the actual browser at 1440×1000 and 1280×800. Do not accept DOM existence alone.

Confirm:

- icon rail remains understandable with labels/tooltips and obvious active tool;
- selected inspector is outside the narrow rail;
- tool name, stable identity, complete anchors and values are readable and not clipped;
- tool-aware line/fill/Text settings, visibility, lock, Fib direction and Text are understandable;
- Apply/Cancel/validation behavior is honest;
- inspector does not cover essential replay/trading controls or reduce the chart below usable size;
- keyboard input inside fields does not trigger chart delete/replay shortcuts;
- compact screenshot shows the inspector open, not merely the chart with no selected properties.

Judge whether the result is genuinely usable for TA practice, not merely testable.

### 7. B3-R06 UAT integrity

Compare baseline Reviewer result `test-results/product-uat/2026-07-18T01-37-13-462Z/results.json` with current result `2026-07-18T03-15-14-250Z/results.json`.

Verify:

- all 174 baseline IDs are present exactly once with the same blocking semantics;
- 42 new IDs are additive and assert real outcomes;
- pointer checks dispatch actual chart gestures and record exact before/after anchor/corner semantics;
- inspector containment checks measure visible geometry and complete values;
- waits do not bypass failed persistence/rendering;
- runtime/provider/request errors and failed API outcomes are observable;
- no hidden hard-coded acceptance surrogates are used as user-visible proof.

### 8. Independent gates

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

The browser harness may require permission to bind localhost. Use its isolated temporary database. Retain the independent artifact directory and inspect the required screenshots. Do not mutate `backend/sumi.db`.

## Reviewer decision

Choose exactly one evidence-backed outcome:

### A. Approve and close Batch 3

Only if B3-R01–B3-R06 and D-01–D-11 are genuinely satisfied:

- append a dated final closure to `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`;
- append Reviewer closure/evidence to the Batch 3 ExecPlan without rewriting DEV history;
- update `docs/INDEX.md` so Batch 3 is approved/closed;
- explicitly state that this approves the Drawing MVP only, not Sumi as a complete professional product;
- record the accepted cross-client-CAS limitation;
- authorize Batch 4 only through a new full standalone DEV prompt described below.

### B. Return Batch 3 for another bounded closure

If any material requirement is unsupported:

- add precise finding IDs, priority, code/artifact evidence and closure criteria to the Batch 3 review;
- create a new full standalone hardening prompt in `docs/dev-prompts/`;
- do not authorize Batch 4;
- do not fix product code in the Reviewer session.

## Requirement for the next DEV prompt

Do not paste a short command into chat. Create the prompt as a repository file and make it fully standalone for a **new DEV session with no conversation history**.

If Batch 3 is approved, create:

`docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md`

That prompt must contain enough context to run safely in a new session, including:

- product purpose and prohibition on unsupported “TradingView-like”/product-complete claims;
- exact repository path/current-checkout/no-branch-worktree-commit-push-merge-reset-clean rules;
- mandatory canonical documents and approved Batch 1–3 reviews/ExecPlans to read;
- explicit statement that Batch 1 Replay foundation, Batch 2 Indicator Manager and Batch 3 Drawing MVP are approved baselines and blocking regressions;
- Batch 4 only; Batch 5 unauthorized;
- creation of `docs/exec-plans/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW.md` before product code;
- outcome and exact acceptance mapping for T-01–T-05 plus R-02/R-04 and all G invariants;
- current workspace audit before design: Replay header, chart area, drawing rail/inspector, indicator manager, trade controls, pending orders, open position, decision journal/checklist, keyboard and responsive behavior;
- a coherent information hierarchy with chart remaining primary, accepted minimum chart geometry at 1440×1000 and 1280×800, and an explicit mobile limitation;
- understandable Buy/Sell/Hold/Skip, pending order, position/P&L, T+2 state/feedback, marker semantics and error/disabled/loading states;
- journal and checklist accessible without route loss or replay-state loss;
- current symbol/date/bar/OHLCV context obvious without crosshair;
- replay navigation synchronizes candles, indicators, drawings, markers, orders, positions and journal with no duplicate/race/future leak;
- reload/resume restoration for complete integrated workspace state;
- keyboard scope/focus rules so chart, drawing, trade and form controls do not fight;
- no broad backend rewrite; backend logic remains out of routes; stop if a verified missing contract requires architecture decision;
- local-first/no telemetry/temp-DB/no production-DB mutation invariants;
- no new UI/chart dependency without an approved decision;
- focused frontend/backend integration tests and additive product UAT;
- manual browser scenarios using real trade/order/journal actions, not hidden state declarations;
- required 1440×1000 and 1280×800 screenshots plus machine state and zero runtime/page/console errors;
- regression preservation for every accepted Batch 1–3 assertion ID;
- exact gates: tests, lint, build, `verify-v2.sh`, `run-product-uat.sh`, `verify-product.sh`, `git diff --check`, DB hash;
- stop/escalation conditions, rollback, artifact/handoff requirements;
- final line `BATCH 4 DEV COMPLETE — STOPPED AT REVIEWER GATE`;
- prohibition on starting Batch 5 or declaring the complete product professional/release-ready.

Before authorizing Batch 4, inspect existing trade/order/journal backend contracts and current UAT baseline enough to avoid writing an aspirational prompt that conflicts with repository reality. The Batch 4 prompt may authorize a scoped backend fix only when the existing contract is demonstrably defective and the ExecPlan records it; it must not silently expand into a backtesting-engine rewrite.

If Batch 3 is not approved, create an equally standalone Batch 3 closure prompt containing the full findings, invariants, exact acceptance and verification requirements. A new DEV session must not need access to this Reviewer conversation.

## Final response from the new Reviewer session

Report in Vietnamese:

```text
BATCH 3 REVIEW COMPLETE

Verdict:
- APPROVED AND CLOSED
or
- RETURNED FOR HARDENING

Independent evidence:
- ...

Findings/accepted limitations:
- ...

Next authorized action:
- ...

Files created/updated:
- ...

Commands run:
- ...
```

Link the review report and the full next DEV prompt file. Do not include the entire DEV prompt in the chat response. Do not start implementation from the Reviewer session.
