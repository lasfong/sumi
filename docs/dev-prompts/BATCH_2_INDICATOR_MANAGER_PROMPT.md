# DEV prompt — Batch 2 Professional Indicator Manager

Continue in the existing Sumi checkout and working tree. Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, retag, discard, or overwrite existing Reviewer/user changes.

This prompt authorizes **Batch 2 only**. Batch 1 is Reviewer-approved and must remain green. Batch 3 drawing work is not authorized.

## Read before editing

Read these files completely, in this order:

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
4. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
5. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
6. `docs/DEVELOPMENT_OPERATING_MODEL.md`
7. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
8. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`, including final closure
9. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`, including final closure
10. Current indicator API, registry, controller, chart pane/series/render code, persistence, and product-UAT harness.

Before product code changes, create and maintain `docs/exec-plans/BATCH_2_PROFESSIONAL_INDICATOR_MANAGER.md` using `PLANS.md`. Record the current baseline, scope, affected modules, acceptance mapping I-01–I-13, risks, rollback, data/state migration policy, exact commands, progress, decisions, deviations, and evidence paths.

## Outcome

Deliver one complete vertical capability: a professional, explicit, persistent Indicator Manager for EMA, RSI, MACD, CCI, and Volume. It must be comfortable to operate in browser UAT, not merely render indicator lines.

Backend `IndicatorEngine` and the session-scoped replay indicator API remain authoritative for calculations. The frontend must not reimplement indicator math and must never request or expose future candles.

## Required architecture

### Indicator product state

Replace the current name/params-only configuration with a versioned, serializable Sumi-owned domain model containing at least:

- stable unique instance ID;
- indicator definition/type ID and human label;
- validated parameters;
- target pane/placement;
- visibility;
- style/color per rendered series where applicable;
- deterministic order;
- state schema version.

Multiple instances of the same type must remain independently addressable. Chart/provider keys and oscillator pane identities must use the stable instance ID, never only the indicator type. Keep UI labels/pane semantics separate from backend dataframe column names.

Define a non-destructive compatibility policy for existing `sumi:workspace:<sessionId>` indicator data. Existing valid EMA/RSI/MACD/CCI/Volume configurations should be promoted where possible; malformed or unknown entries must fail safely without crashing or silently contaminating another session. Document rollback consequences. Do not migrate or rewrite legacy backend drawing records.

### Ownership boundaries

- `ReplayWorkspace` owns composition and visible manager/pane chrome.
- The replay controller coordinates domain state, request lifecycle, persistence, and chart commands; do not move a new page monolith into another file.
- Indicator domain/repository/request code must not depend on React rendering or Lightweight Charts native objects.
- `CandleChart`, pane/series managers, and render registry own Lightweight Charts v5 calls.
- Persistence must store Sumi domain state, never Lightweight Charts/provider-native objects.
- Keep the accepted Batch 1 drawing provider ownership and lifecycle unchanged unless a narrowly required compatibility edit is documented and regression-tested.

### Request lifecycle

Indicator requests must be session scoped and no-future-safe. Add cancellation or stale-result protection and deduplicate equivalent in-flight work so rapid replay navigation, settings edits, visibility changes, reload, or unmount cannot apply obsolete results, duplicate series, or log avoidable errors. One failed indicator must show recoverable feedback without destroying other active indicators.

## Required product behavior

Implement and prove every acceptance criterion I-01 through I-13:

1. An always-visible active indicator list identifies type, parameters, pane, visibility, and color/style.
2. A clear Add Indicator flow supports search/category browsing and parameter entry **before** confirmation.
3. Each indicator instance has an obvious one-action remove control.
4. Each instance can be shown/hidden without losing settings.
5. Settings can be reopened, edited, validated from backend registry metadata, applied, and cancelled without accidental mutation.
6. Multiple same-type instances with different parameters are visually distinguishable and independently removable/editable/toggleable.
7. Full state, ordering, visibility, parameters, styles, placement, and stable IDs restore after reload/resume without duplicates.
8. Every oscillator pane has clear title, legend/current values, settings, visibility, and close controls plus a usable default height.
9. Implement real resize/reorder if the current official API and architecture support it safely; otherwise record and implement an explicitly accepted fixed responsive layout that remains usable at both required viewport sizes. Do not claim unsupported interactions.
10. MACD visibly distinguishes MACD line, signal line, histogram, zero reference, names, and non-overlapping current values.
11. RSI uses a sensible 0–100 scale and visible 30/50/70 reference lines.
12. CCI shows visible -100/0/100 reference lines.
13. Warmup/null/partial data creates no invalid points/segments, misleading active-empty state, console error, or crash.

Volume must participate in the same explicit manager lifecycle even if its pane/style semantics differ from oscillators. Price overlays such as EMA must stay on the price pane and must not interfere with drawing input boundaries.

Use accessible buttons, labels/tooltips, keyboard-safe dialogs/forms, visible selected/disabled/loading/error states, and deterministic focus/close behavior. Do not solve the batch with a collection of unlabeled icon buttons or hidden hover-only controls. Avoid a broad visual redesign outside the replay/indicator surfaces.

## Required focused tests

Add focused tests that prove domain and integration behavior, including at minimum:

- stable unique IDs and two independently managed same-type instances;
- backend-registry defaults and min/max/type validation;
- add/apply/cancel settings semantics;
- remove and visibility without settings loss;
- deterministic order and full persistence/reload promotion;
- per-instance pane/series keys and cleanup with no orphan series/panes;
- request deduplication, stale-response rejection/cancellation, failure isolation, and unmount cleanup;
- null/warmup filtering for every supported render shape;
- correct MACD line/signal/histogram mapping and references;
- RSI/CCI scale/reference policies;
- regression coverage for replay advance/rewind, Horizontal Line, provider destroy/remount, and no-future session API usage.

Do not encode implementation details as the only proof of usability. Browser UAT is mandatory.

## Product UAT additions

Extend `scripts/product-uat.mjs` with semantic assertions for I-01–I-13. Do not delete, weaken, rename away, or relabel any existing assertion. Indicator failures should turn green only after the actual product behavior is exercised. Existing out-of-scope drawing/legacy gaps remain visible failures, not silently excluded from machine results.

At minimum, UAT must independently exercise:

- open the add flow, search/browse, configure and add EMA, RSI, MACD, CCI, and Volume;
- verify always-visible active list and pane chrome;
- add two EMA or two RSI instances with different periods and prove independent identity/actions;
- edit a period, cancel a second edit, toggle visibility, remove one instance, undo no unrelated drawing state, and verify exact visible state;
- navigate replay backward/forward and rapidly change position while requests are in flight;
- verify MACD components/zero, RSI 30/50/70, and CCI -100/0/100 semantically, not by screenshot alone;
- verify warmup/null behavior at an early replay index;
- reload/resume and assert exact IDs, parameters, order, visibility, styles, panes, and no duplicate series;
- confirm Horizontal Line create/select/move/cancel/persistence regression remains green;
- run ten mount/unmount cycles and prove no duplicate listeners, requests, series, or runtime errors;
- capture and review evidence at 1440×1000 and 1280×800 with the manager open and representative panes visible.

The final run must use the isolated temporary UAT database. Report every remaining failed assertion by ID and scope. `blockingFailed` for Batch 2 must be zero, but the total product suite need not be all green because Batch 3/4 gaps remain.

## Regression and verification gates

Record the production DB SHA-256 before work. Run, at minimum:

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

Add focused backend tests only if required to prove the existing registry/session indicator contract; keep business logic out of FastAPI routes. Production `backend/sumi.db` must remain byte-for-byte unchanged. Retain machine-readable UAT output and screenshots.

## Stop and escalate instead of guessing

Stop at Reviewer gate with a documented decision request if any of these is true:

- registry metadata cannot express the required defaults/type/range validation;
- the session indicator endpoint cannot safely distinguish or serve multiple instances;
- satisfying the UI appears to require frontend indicator calculation or sending future candles;
- a Lightweight Charts pane/scale limitation makes I-08–I-12 materially impossible;
- the batch is estimated to exceed ten focused developer-days or requires a broad chart-library replacement;
- a new dependency, backend contract change, database migration, or acceptance-criteria change appears necessary.

Do not install a community indicator/drawing package. Do not begin Trendline, Ray, Rectangle, Fibonacci, Text, magnet, drawing backend migration, Batch 3, or Batch 4. Do not perform unrelated cleanup or cosmetic redesign.

## Final DEV handoff

Update the ExecPlan with decisions, progress, exact verification output, evidence paths, remaining failures, deviations, and self-review. End with:

`BATCH 2 DEV COMPLETE — STOPPED AT REVIEWER GATE`

Report:

- acceptance result for each I-01–I-13;
- architecture/domain/persistence/request-lifecycle changes;
- exact focused/full test counts;
- final UAT pass/fail/`blockingFailed` counts and every retained gap ID;
- console/page/runtime error count;
- 1440×1000 and 1280×800 artifact paths;
- production DB before/after SHA-256;
- deviations and known limitations.

Do not claim Sumi is professionally usable or product-complete. Do not start Batch 3. Stop for Reviewer inspection.
