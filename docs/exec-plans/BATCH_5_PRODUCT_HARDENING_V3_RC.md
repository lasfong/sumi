# Batch 5 — Product hardening and V3 release-candidate evidence

## Outcome

Produce a reproducible, local-only V3 release-candidate evidence set for sustained replay practice, performance/memory stability, migration/backup/restore and core keyboard/accessibility behavior. Preserve every accepted Batch 1–4 capability and all 254 accepted product-UAT IDs. This is a DEV evidence handoff only; it does not self-approve the release, create a tag, or declare the product finished.

## Context and problem

- Authority: `docs/dev-prompts/BATCH_5_PRODUCT_HARDENING_V3_RC_PROMPT.md`; acceptance authority: G-01–G-05, R-01–R-05, I-01–I-13, D-01–D-11 and T-01–T-05.
- Batch 1–4 final Reviewer closures are approved. The immutable browser baseline is `test-results/product-uat/2026-07-18T15-23-28-222Z/results.json`: 254 passed, 0 failed, 0 blocking failed, 254 unique IDs, empty runtime/provider/indicator-request failure arrays and two deliberately bounded practice-rejection console messages.
- Existing deterministic product UAT proves the complete short workflow and both supported viewports, but it does not provide 30 elapsed minutes of uninterrupted use, repeatable performance/memory samples, a restored-copy workflow, or a consolidated accessibility/release evidence ledger.
- Checkout provenance before Batch 5: direct dirty `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; protected `v2.0.0-rc2^{}` at `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`. Production `backend/sumi.db` SHA-256 is `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

## In scope

- One uninterrupted real-browser practice run lasting at least 1,800 seconds on deterministic temporary data, with timestamped actions spanning replay keyboard/±5/autoplay/pause/speed, all indicator lifecycle operations, all drawing tools/edit/history/magnet, trading/LIMIT/T+2/CLOSE, journal, rewind/forward and reload/resume.
- Repeatable performance/memory measurement at 1440×1000 and 1280×800 using long visible history, the five-indicator representative maximum, at least 50 persisted drawings, navigation/autoplay/pan/zoom, ten route remounts and reload cleanup.
- Temporary-database fresh migration, production-format-copy migration/open, SQLite backup and restored-copy verification, plus local workspace export/import checks for indicator and drawing documents and retained workflow semantics.
- Core keyboard/accessibility inspection through real browser input and dependency-free DOM/computed-style checks. Add product-UAT assertions only where the deterministic short gate can prove the contract honestly.
- V3 acceptance matrix, RC notes, exact verification/recovery instructions and evidence index without rewriting historical V2/V3 review evidence.

## Out of scope

- New trading, chart, indicator, drawing or journal features; architecture/provider/chart replacement; mobile-first functionality; telemetry; cloud backup/sync; production database migration; cross-client drawing CAS; disaster-recovery claims beyond the verified local copy procedure; release tag/commit/push or post-V3 work.
- Performance optimization without a reproducible P0/P1 budget failure. A demonstrated defect must receive red evidence before a narrow fix; any fix requiring a new capability, dependency or architecture decision stops for Reviewer.

## Invariants

- No future candle is sent to the browser. Every sustained/performance checkpoint compares API candle length, chart-visible input, marker/indicator/drawing inputs and `current_index`.
- Backend `IndicatorEngine` remains authoritative. Drawing documents remain Sumi-owned schema v1; provider-native JSON is never backup authority.
- Every automated run uses a fresh or copied temporary database. `backend/sumi.db` is read-only and its before/after hash must match.
- No market, trade or journal data leaves localhost. Harness network auditing permits only loopback application traffic and browser-internal URLs.
- All 254 accepted IDs remain exactly once with unchanged names, pass values and prefix-based blocking semantics. New product-UAT IDs use unique blocking `batch5.*` names.

## Current architecture

- `scripts/run-product-uat.sh` provisions an isolated SQLite database and local FastAPI/Vite services, while `scripts/product-uat.mjs` performs the accepted short 254-check browser workflow.
- Replay state is split between temporary SQLite workflow records, an opaque backend drawing mirror and browser localStorage indicator/drawing documents. Backup therefore requires both a consistent SQLite copy and an explicit local workspace JSON export.
- No automated accessibility dependency is installed. Existing controls already use roles/labels/test IDs, so Batch 5 can inspect semantics and exercise focus with Playwright without changing the dependency/license surface.

## Target design

```text
run-batch5-hardening.sh
  -> fresh migrated temporary SQLite + deterministic seed
  -> local FastAPI/Vite only
  -> batch5-hardening-uat.mjs
       sustained 30-minute real-UI timeline
       performance/memory samples and budgets
       keyboard/accessibility checks
       workspace/database semantic snapshot
  -> stop services and create consistent local backup bundle
  -> restored temporary SQLite + imported workspace JSON
  -> restore verification mode and semantic equality
  -> migration/recovery manifest and retained logs

short product gate
  -> existing 254 IDs unchanged
  -> additive batch5 accessibility/keyboard assertions
```

Harnesses write partial results in `finally` blocks so failed sustained/performance/accessibility runs retain timestamps, measurements, screenshots and error arrays.

## Performance and memory budgets

Budgets are fixed before measurement and apply to both supported viewports. Measurements use one warmup followed by at least five samples; report median, p95 where meaningful and worst case.

| Measure | Budget | Method |
| --- | ---: | --- |
| Initial replay workspace usable | median ≤ 5,000 ms; worst ≤ 8,000 ms | navigation start until header, chart state and practice snapshot are available |
| Replay `Next`, `Prev`, `+5`, `-5` response | median ≤ 500 ms; p95 ≤ 900 ms; worst ≤ 1,500 ms | input timestamp to exact synchronized `current_index`/API/chart state |
| Indicator HTTP latency | median ≤ 1,500 ms; p95 ≤ 2,500 ms; worst ≤ 4,000 ms | browser request/response timing for session indicator endpoints |
| Animation responsiveness | RAF-gap p95 ≤ 100 ms; worst ≤ 500 ms | in-page `requestAnimationFrame` observer during bursts/autoplay/pan/zoom |
| Long tasks | no task > 1,000 ms; ≤ 20 tasks ≥ 50 ms per 10 measured minutes | `PerformanceObserver('longtask')` where supported |
| Heap growth after ten remounts/reload cleanup | ≤ 20 MiB and ≤ 35% over post-GC/warm baseline | Chromium CDP `Performance.getMetrics`; force GC only at matched checkpoints |
| DOM growth after cleanup | ≤ 150 nodes and ≤ 20% | `document.getElementsByTagName('*').length` at matched route state |
| Drawing/provider ownership | exactly 1 primitive and 6 listeners; no growth | provider interaction snapshot before/after churn |
| Indicator chart ownership | exact stable visible instance/series/pane counts; no duplicate keys | chart diagnostic snapshot before/after churn |
| Request duplication | zero simultaneous duplicate GETs for the same normalized indicator URL; no stale failed request | request lifecycle tracker |

Budget failure is blocking evidence. Optimize only after reproducing it independently; stop if compliance requires provider/chart/architecture replacement.

## Accessibility and keyboard budgets

- Every core interactive element in Replay has a nonempty accessible name; duplicate names are allowed only when scoped by distinct labelled regions/stable instance identity.
- Tabs expose `tab`/`tabpanel` relationships and selected state. Dialogs have an accessible name, keep focus inside during Tab/Shift+Tab, close on Escape where specified and return focus to the opener.
- Every visible focusable core control has a visible focus indicator whose computed outline/box-shadow changes on `:focus-visible`; no keyboard path requires pointer-only activation.
- Text/editable controls isolate replay arrows/Space/Delete/Backspace/Escape as documented; the same shortcuts work outside editors. Disabled historical and sub-1180 trade states are represented by actual disabled controls and explanatory status text.
- Core text/control contrast uses WCAG AA thresholds: 4.5:1 for normal text, 3:1 for large text and UI boundaries, measured from computed foreground/background colors where backgrounds are opaque. Browser zoom at 200% must retain access to replay, Trade and Journal without horizontal clipping of the entire application; accepted internal indicator-strip overflow remains documented.

No accessibility dependency will be added. If dependency-free inspection cannot verify a requirement, record it for manual Reviewer inspection rather than fabricating a pass.

## Affected files

- New harnesses: `scripts/run-batch5-hardening.sh`, `scripts/batch5-hardening-uat.mjs`, and a narrow temporary-database recovery verifier under `backend/scripts/` if shell/Node verification is insufficient.
- Additive short-gate evidence: `scripts/product-uat.mjs`; product code changes only after a red P0/P1 reproduction.
- Documentation: this ExecPlan, `docs/V3_ACCEPTANCE_MATRIX.md`, `docs/V3_RELEASE_CANDIDATE_NOTES.md`, `docs/V3_VERIFICATION_AND_RECOVERY.md`, `docs/V3_EVIDENCE_INDEX.md`, and `docs/INDEX.md` only if a canonical link is missing.
- Retained evidence under `test-results/batch5-hardening/<run-id>/` and the existing product-UAT artifact root.

## Milestones

1. **Plan/baseline:** complete authority reading, dirty-tree/provenance/hash/baseline audit and this pre-change plan.
2. **Sustained/performance harness:** fail-safe artifacts, real elapsed-duration timeline, no-future/duplicate checks and fixed-budget measurements at both viewports.
3. **Migration/recovery:** fresh/copy migration, backup manifest, restored database/workspace equality and malformed/legacy compatibility evidence using temporary copies only.
4. **Accessibility/keyboard:** real keyboard and dependency-free semantic/focus/contrast/zoom checks; red-before-fix for any P0/P1.
5. **Release ledger:** complete G/R/I/D/T matrix, RC notes, reproducible verification/recovery guide, evidence index and known limitations.
6. **Final gates:** full tests/UAT/product gate, 254-ID comparison, manual screenshot review, DB/provenance/dirty-scope audit and Reviewer handoff.

## Acceptance mapping

| Acceptance IDs | Implementation/evidence | Verification |
| --- | --- | --- |
| G-01–G-03 | Required technical gates, fail-safe isolated artifacts, backup manifests | exact commands/results and unchanged production DB |
| G-04 | fixed budgets, red-before-fix rule, zero unresolved demonstrated P0/P1 | sustained/performance/accessibility results |
| G-05 | localhost-only request audit, no telemetry/dependency | network origins and source/dependency audit |
| R-01–R-05 | continuous no-future checks, all navigation modes, synchronized state, churn/reload | sustained timeline and short product UAT |
| I-01–I-13 | representative five-indicator set, lifecycle/churn/request/restore/warmup | sustained/performance state and accepted IDs |
| D-01–D-11 | all tools, edit/history/magnet/persistence, many-drawing/churn cleanup | sustained actions, performance ownership and restore equality |
| T-01–T-05 | uninterrupted practice/trade/journal/T+2/rewind/resume at both viewports | 30-minute timeline, final state and screenshots |

## Verification commands

```bash
git diff --check
cd backend && ../.venv/bin/python -m pytest -q
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
./scripts/run-batch5-hardening.sh
shasum -a 256 backend/sumi.db
git rev-parse HEAD
git rev-parse 'v2.0.0-rc2^{}'
```

The Batch 5 runner must also retain the exact fresh migration, copied-production-format migration, backup, restore and accessibility subcommands in its manifest.

## Rollback and compatibility

- Harness/documentation rollback removes only Batch 5 scripts/docs/artifacts; it changes no persisted schema or production data.
- Any narrow product fix is separately recorded with its red evidence and source-only rollback. No backup/restore procedure writes in place: stop services, copy the SQLite database, export local workspace JSON, verify hashes, restore into a new path, then start against the restored copy.
- `v2.0.0-rc2` is a protected code/data compatibility boundary. V3 schema-v1 indicator/drawing/checklist data is not promised readable by RC2; rollback instructions preserve a backup and explain that code rollback must not destructively rewrite newer opaque/local records.

## Risks and mitigations

- Thirty-minute automation can hide idle gaps: retain every timestamp/action and enforce minimum elapsed wall-clock duration; perform meaningful actions throughout all six five-minute windows.
- Headless performance variance: warm up, collect multiple samples, report median/p95/worst and use conservative budgets rather than one sample.
- SQLite copy consistency: stop the temporary service or use SQLite online backup before copying; hash source/backup/restored files and run semantic queries on the restored service.
- Browser-local persistence omitted from DB backup: export/import exact localStorage keys and compare canonical indicator/drawing documents separately.
- Accessibility false confidence: combine DOM semantics, computed styles and real keyboard traversal; record manual-only limitations honestly.

## Stop conditions

- Stop for Reviewer if a P0/P1 budget/accessibility/data-integrity failure requires a new feature, dependency, provider/chart replacement, backend contract/schema migration, unsafe in-place operation, acceptance weakening or reinterpretation of retained data.
- Stop if the 30-minute run cannot remain on real UI/local deterministic services, if any future candle appears, if restored semantic state is ambiguous/lost, or if a core accessibility P0/P1 remains unresolved.
- Stop before release tagging, self-approval, production DB mutation, external transmission or post-V3 work.

## Review closure plan — B5-R01–B5-R05

### Bounded scope and red evidence

- Closure authority is `docs/dev-prompts/BATCH_5_REVIEW_CLOSURE_PROMPT.md`; work is limited to the five findings in `docs/reviews/BATCH_5_REVIEW_2026-07-19.md`. The canonical comparison baseline is the returned 272-ID result at `test-results/batch5-hardening/2026-07-19T01-05-23Z/product-uat/2026-07-19T01-05-26-343Z/results.json`.
- B5-R01 red browser evidence is already retained in that canonical timeline: an ArrowRight action whose only recorded purpose was `keyboard-tab-navigation` changed the authoritative replay index from 70 to 71 at iteration 5 and from 72 to 73 at iteration 15. A focused regression test will reproduce the same event path before the guard is changed.
- B5-R02 red review evidence is structural: `verifyNoFuture()` records only practice/session index, API candle length and marker dates, despite the invariant naming chart, indicator and provider/drawing inputs. Controlled negative fixtures will demonstrate that each omitted surface currently escapes the claimed final predicate.
- B5-R03 red review evidence is structural: `batch5-manifest.mjs` derives pass without validating the referenced product result, while the baseline audit does not require additive checks to pass. Negative fixtures will demonstrate the fail-open cases before the strict validators replace them.
- B5-R04 red evidence is the returned bundle's `focusStyle` (a 3px black outline with no before-state or adjacent-background contrast) and 200% geometry (`scrollWidth: 2880`, `clientWidth: 1440`) checked only with `isVisible()`. Closure will measure actual focus delta/contrast, intersections, scroll axes and keyboard activation; a responsive redesign remains a stop condition.
- B5-R05 red evidence is the retained 69–96-bar sustained range, untimestamped short-workflow categories and reload-only churn. Closure will timestamp the real core workflow, enforce category/window coverage, use a genuinely long authoritative history and put ten analytics/replay remounts plus reloads inside matched measurements.

### Affected modules and design

- Product shortcut policy: `frontend/src/features/replay/` (one reusable eligibility function), `frontend/src/components/replay/ReplayWorkspaceController.tsx`, with `PracticeRail` propagation only as defense in depth and focused tests under `frontend/src/components/replay/__tests__/`.
- Browser closure evidence: `scripts/product-uat.mjs` plus a small dependency-free closure/self-test module where pure no-future and evidence predicates can be exercised negatively. Existing 272 IDs remain byte-semantically unchanged; new blocking assertions use unique `batch5.closure.*` IDs.
- Fail-closed bundle: `scripts/batch5-evidence-audit.mjs`, `scripts/batch5-manifest.mjs`, `scripts/run-batch5-hardening.sh`, and retained negative-fixture output. Every cited canonical file is present and SHA-256 recorded; result/error/migration/recovery/restore/database/provenance predicates are conjunctive.
- Evidence ledger only after a fresh full closure: `docs/V3_ACCEPTANCE_MATRIX.md`, `docs/V3_RELEASE_CANDIDATE_NOTES.md`, `docs/V3_VERIFICATION_AND_RECOVERY.md`, `docs/V3_EVIDENCE_INDEX.md`, and this ExecPlan. The returned bundle remains historical and is not deleted.

### Acceptance mapping

| Finding / acceptance | Closure implementation | Blocking evidence |
| --- | --- | --- |
| B5-R01 / R-03, D-05–D-06, T-03 | Central default-prevented/interactive/editable/modal eligibility policy; component propagation defense | Byte/ID/index equality for tabs, buttons, dialogs, forms, contenteditable and inspector; intended background shortcuts exactly once |
| B5-R02 / R-01, R-04, I-13, D-07 | Full authoritative future-boundary snapshot at every checkpoint and visible-provider filtering | Per-surface index/count/max-date fields, fail-fast live/partial divergence, one negative case per surface |
| B5-R03 / G-01–G-03 | Strict 272 baseline and additive-result audit; checksummed self-validating manifest | All requested negative fixtures exit nonzero; migrations/recovery/restore/DB/HEAD/tag exact |
| B5-R04 / G-04, T-01, T-03 | Focus before/after contrast and focused screenshot; 200% bounding/scroll/reflow/tab activation | Blocking geometry and keyboard evidence at the required viewport, or Reviewer stop with exact conflict |
| B5-R05 / R-03–R-05, I-01–I-13, D-01–D-11, T-02–T-05 | Timestamped complete workflow, six-window mechanical coverage, long history and matched route-remount/reload stress | At least 1,800 real seconds; complete category/window matrix; long-history count/date; stable matched ownership/request metrics |

### Rollback and compatibility

- Rollback removes only the closure shortcut helper/tests and harness/audit/document changes. It introduces no persisted schema, dependency, telemetry, external request or provider/backend architecture change.
- All runs use fresh/copied temporary SQLite paths. `backend/sumi.db`, HEAD and `v2.0.0-rc2^{}` are read-only provenance inputs and must match their recorded expected values before and after the final run.
- If 200% access requires a nontrivial responsive redesign, or any future-boundary/recovery mismatch cannot be resolved inside these existing boundaries, stop for Reviewer rather than weakening an assertion.

### Exact closure verification commands

```bash
git diff --check
cd frontend && npm test -- --run src/components/replay/__tests__/PracticeWorkflow.test.tsx
node scripts/batch5-evidence-negative-selftest.mjs
cd backend && ../.venv/bin/python -m pytest -q
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
./scripts/run-batch5-hardening.sh
shasum -a 256 backend/sumi.db
git rev-parse HEAD
git rev-parse 'v2.0.0-rc2^{}'
```

## Progress log

- 2026-07-18: read the Batch 5 prompt and all mandatory canonical/review/Batch 1–4 ExecPlan sources in order. Confirmed Batch 4 final Reviewer approval and dedicated Batch 5 authorization.
- 2026-07-18: inventoried the preserved dirty tree, baseline 254 unique passing IDs, dependencies, branch/HEAD/tag and production DB hash. Defined sustained-session, performance/memory, accessibility, migration/backup/restore, rollback and stop policies before any product or harness change.
- 2026-07-19: added red browser/focused evidence and bounded fixes for modal/tab keyboard access, external font traffic and post-reload WebSocket command fallback. Added fail-safe sustained/performance/recovery harnesses without a new dependency.
- 2026-07-19: canonical hardening run `2026-07-19T01-05-23Z` passed 272/272, including 1,800.137 seconds, 115 no-future checkpoints, 119 actions, 11 reloads, both viewports, 50 drawings, fixed budgets, backup/restore and unchanged production DB hash.
- 2026-07-19: reviewed retained 1440×1000, 1280×800, 200% and sustained stress screenshots; recorded accepted internal indicator-strip overflow, stress-state density and manual composited-contrast limitation.
- 2026-07-19: final gates passed after the last product fix: `git diff --check`; backend 97 passed/1 skipped; frontend 116 passed; lint/build; `verify-v2.sh`; standalone UAT 263/263 at `2026-07-19T02-01-09-866Z`; and `verify-product.sh` with UAT 263/263 at `2026-07-19T02-02-39-490Z`.
- 2026-07-19: review closure implemented the centralized global-shortcut policy and full future-boundary/chart/indicator/provider diagnostics; strict 272-ID audit and checksummed manifest; 19 negative audit/manifest/future-surface fixtures; timestamped category/window coverage; long-history and route-remount matched measurement design. Frontend 128/128, lint and build passed.
- 2026-07-19: focused browser UAT `test-results/product-uat/2026-07-19T03-05-48-899Z/results.json` proved `batch5.closure.R01.keyboard-isolation` green across tabs, buttons, dialogs, forms, contenteditable, drawing inspector and exact background actions. It also activated the R04 stop condition under inline CSS zoom: the document became 2880×2000 inside a 1440×1000 viewport, effective body width was 720px, horizontal overflow was hidden, and the direct-Tab search did not focus Journal. The run stopped before a canonical 1,800-second rerun and awaited Reviewer classification.
- 2026-07-19: corrected R04 evidence passed in focused UAT `2026-07-19T04-31-11-883Z` and full product gate `2026-07-19T04-33-37-972Z`, both 265/265. The first canonical closure attempt then failed closed at its first sustained no-future checkpoint: a one-step WebSocket navigation exposed host-timezone conversion of a naive daily candle timestamp, shifting 2023-12-22 to 2023-12-21 in the chart. The UTC wire conversion fix has two regression tests (backend now 98 passed), and a 120-second sustained smoke proved API, chart, all five indicators and provider state agree at every sampled boundary; its only failures were the intentionally unmet 1,800-second/sample-count budgets.
- 2026-07-19: the next full run retained 1,955 seconds but correctly failed R05 because system sleep left four five-minute windows empty; it also exposed expected cancellation being logged as an API error and compound remount+reload measurement. Focused accelerated churn then drove the acknowledged manual-Next path, cancellation classification, orphaned indicator-work cancellation and interval-based duplicate measurement to 277/277 before the new canonical run.
- 2026-07-19: canonical closure bundle `test-results/batch5-hardening/2026-07-19T05-41-28Z/manifest.json` passed 277/277 with the 272 returned IDs unchanged plus exactly five closure IDs, 1,800.139 seconds, 134 actions, 124 no-future samples, all six windows active, 10 matched route remounts, 11 reloads, 19/19 negative cases, exact recovery and unchanged DB/HEAD/tag provenance. Final diff gates passed: backend 99/1, frontend 129, lint/build, `verify-v2`, standalone UAT 265/265 at `06-14-06-352Z`, and `verify-product` UAT 265/265 at `06-15-59-577Z`.

## Decision log

- Use the installed Playwright/Chromium and browser Performance/CDP APIs; add no accessibility or performance dependency.
- Keep the accepted short product UAT as regression authority and add a separate elapsed-time hardening runner. A short test cannot substitute for the required 1,800 seconds.
- Treat backup as a bundle of a consistent SQLite copy plus explicit browser-local workspace JSON. Neither alone preserves the complete local-first workspace.
- A legacy production-format copy with application tables but no Alembic metadata is schema-validated, stamped and upgraded only on the temporary copy; production is never stamped in place.
- Manual one-step Next uses the acknowledged HTTP mutation; WebSocket remains the autoplay transport. A rapid route-remount smoke proved that `readyState=OPEN` alone cannot acknowledge a command while the old socket is closing, so an unacknowledged manual navigation is not an acceptable fallback boundary.
- Global replay/drawing shortcuts use one target/default-prevented eligibility policy. PracticeRail propagation is defense in depth; accepted harness shortcuts explicitly focus the non-interactive chart background.
- Retained drawing documents may contain anchors after a rewound boundary, but `providerVisible` is true only when every rendered anchor has a current chart coordinate. Continuous evidence validates visible provider dates and retained-future filtering separately.
- R04 was escalated rather than weakened. Reviewer subsequently classified inline CSS zoom and direct Tab-to-Journal as invalid proxies for browser reflow and ARIA roving-tab navigation; the continuation decision below controls the corrected method.
- Daily candle timestamps on the WebSocket wire are interpreted as UTC when database values are timezone-free. Python's host-local conversion is invalid for these date-key candles because it can change the calendar date and make the chart overwrite the preceding bar; aware timestamps continue to preserve their represented instant.

## Review closure stop evidence — 2026-07-19

- Blocking artifact: `test-results/product-uat/2026-07-19T03-05-48-899Z/results.json` — 262 passed, 3 failed, all three limited to the existing focus/zoom assertion and the additive `batch5.closure.R04.accessibility-evidence`; R01 closure is green and no unrelated accepted ID failed.
- Exact 200% geometry: document `scrollWidth/clientWidth = 2880/1440`, `scrollHeight/clientHeight = 2000/1000`, body `clientWidth = 720`, `overflow-x: hidden`, app bounds `left=-792`, `right=2088`. Replay and Trade intersect after keyboard traversal; Journal is at `x=2428`, `y=-469`, does not intersect and is not focused after 120 Tabs.
- This activates the prompt's explicit Reviewer stop condition. `run-batch5-hardening.sh`, the final 1,800 seconds, final manifest and evidence-ledger repointing were not run because they cannot honestly pass R04 without a nontrivial responsive change. The returned canonical bundle remains the evidence index target until Reviewer authorizes a reflow scope or changes the compatibility decision.

## Original Batch 5 completion evidence (returned)

- Historical DEV evidence bundle: `test-results/batch5-hardening/2026-07-19T01-05-23Z/manifest.json` (`pass: true`), subsequently returned by the Reviewer findings below and superseded for closure by the final DEV handoff at the end of this plan.
- Product result: 272 passed, 0 failed, 0 blocking failed; baseline audit preserves 254/254 IDs and reports 18 additive Batch 5 IDs.
- Performance: usable median/worst 911/947 ms; navigation median/p95/worst 153/164/166 ms; indicator 33/55/86 ms; RAF p95/worst 17.9/84.4 ms; heap -3.07 MiB; DOM +12; provider 1 primitive/6 listeners.
- Recovery: SQLite semantic equality and exact practice/indicator/drawing/journal restored-browser comparisons; production DB before/after SHA-256 `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Batch 5 remains a DEV handoff pending independent Reviewer inspection. No release approval or tag is asserted here.

## Reviewer gate — 2026-07-19

Status: **RETURNED FOR ONE BOUNDED CLOSURE. V3 release approval remains unauthorized.**

The Reviewer accepted the retained 1,800-second run, numerical performance results, backup/restore equality, local-only traffic, documentation and green existing gates, but found five release blockers:

- B5-R01: PracticeRail Arrow navigation bubbles to the global Replay handler and advances the replay index; the submitted timeline demonstrates this at iterations 5 and 15.
- B5-R02: continuous no-future checkpoints omit chart, indicator and visible provider/drawing inputs named by the invariant.
- B5-R03: the canonical manifest can report pass without requiring the referenced Batch 5 product checks to pass.
- B5-R04: focus and 200% assertions do not prove visible contrast, viewport reachability or the recorded no-clipping/reflow contract.
- B5-R05: sustained/performance measurement remains near 69–96 visible bars, omits timestamped core trade/journal/drawing-edit actions, and does not include matched route-remount churn.

Independent gates passed: backend 97/1, frontend 116, lint/build, `verify-v2.sh`, and `git diff --check`. Production DB, HEAD and protected tag remained unchanged. Closure authority is `docs/dev-prompts/BATCH_5_REVIEW_CLOSURE_PROMPT.md`. Preserve all 272 IDs, rerun a full 1,800-second canonical closure, and stop again at Reviewer gate.

## Reviewer R04 continuation decision — 2026-07-19

- Reviewer confirmed that stopping was correct, but rejected the conclusion that a responsive redesign had been demonstrated. Inline CSS `zoom: 2` retained the 1440px media-query viewport, while Journal's `tabIndex=-1` was the expected inactive state of the ARIA roving tablist.
- The failing focused result predates the current focus CSS by about one minute, so the current rule requires fresh browser verification.
- Authorized method: use an effective 720×500 CSS layout viewport for the 1440×1000-at-200% reflow case; prove the 768px media query is active; reach Trade by sequential Tab and Journal by ArrowRight; verify focus/selection/panel/bounds and application horizontal containment. Vertical reflow is permitted.
- Authorized product scope, only if that corrected test remains red: narrow CSS containment/reflow inside the existing small-viewport breakpoint. Preserve the 1180px full-workstation threshold, disabled sub-1180 trade behavior, desktop layouts and all architecture/provider boundaries. Broad responsive redesign remains unauthorized.
- Continuation authority is `docs/dev-prompts/BATCH_5_R04_CONTINUATION_PROMPT.md`. It supersedes the earlier closure-authority sentence above. After focused R04 green, run all final gates and one new canonical 1,800-second session. Expected canonical identity is 272 preserved baseline IDs plus exactly five blocking closure IDs, 277/277.

## R04 continuation execution plan — 2026-07-19

- Scope: correct only the R04 measurement according to `docs/dev-prompts/BATCH_5_R04_CONTINUATION_PROMPT.md`; retain the already-green R01 implementation and the implemented R02/R03/R05 harnesses unless a focused/final gate exposes a concrete defect.
- Method: replace inline CSS zoom with a 720×500 CSS layout viewport at device scale factor 2, recording the CSS viewport, DPR, physical 1440×1000 reference and `matchMedia('(max-width: 768px)')`. Sequential Tab targets the selected Trade tab; ArrowRight exercises the ARIA roving-tab contract to Journal. Replay is traversed independently. Bounding/intersection evidence covers document, body, app, controls and active panel; vertical scrolling/reflow is allowed and application-wide horizontal clipping fails.
- Focus method: compare unfocused and keyboard-focused outline/box-shadow for representative Replay, PracticeRail and dialog/opener controls, composite transparent ancestor backgrounds as alpha zero, require a changed indicator with at least 3:1 adjacent-background contrast, and retain the focused screenshot.
- Product boundary: preserve the 1180px full-workstation threshold and disabled/explained sub-1180 trade state. If corrected measurement remains red, only narrow containment/reflow CSS inside the existing ≤768px breakpoint is authorized; broader responsive/navigation/architecture/provider work remains a stop condition.
- Rollback: remove only the R04 device-metrics/traversal/contrast harness correction and any demonstrated narrow ≤768px CSS containment fix. No persistence, database, dependency or desktop layout changes are involved.
- Focused command: `./scripts/run-product-uat.sh`; inspect `batch5.accessibility.visible-focus`, `batch5.accessibility.zoom-200-core-access`, `batch5.closure.R04.accessibility-evidence`, `22-batch5-zoom-200.png` and `23-batch5-focused-control.png`.
- After focused green, exact final commands remain: `git diff --check`; backend full pytest; frontend full test/lint/build; `./scripts/verify-v2.sh`; `./scripts/run-product-uat.sh`; `./scripts/verify-product.sh`; `node scripts/batch5-evidence-negative-selftest.mjs`; `./scripts/run-batch5-hardening.sh`; production DB SHA-256; exact HEAD and `v2.0.0-rc2^{}`. Final canonical identity must be 277/277: the preserved 272-ID baseline plus exactly five blocking `batch5.closure.*` IDs.

## Review closure DEV handoff — 2026-07-19

- Canonical manifest: `test-results/batch5-hardening/2026-07-19T05-41-28Z/manifest.json` (`pass: true`). Product result is 277 passed, 0 failed, 0 blocking failed, 277 unique IDs. Baseline audit preserves all 272 returned IDs and admits exactly R01–R05 as additive blocking checks.
- Sustained scope: 1,800.139 seconds, 134 timestamped actions, 124 full-surface future-boundary samples, 50 drawings, long history at 254 visible candles, 10 distinct matched Analytics/Replay remounts, 11 reloads and meaningful actions in each five-minute window (24/21/22/21/22/20).
- Performance: navigation median/p95/worst 148/158/161 ms; workspace median/worst 909/954 ms at both viewports; indicator median/p95/worst 40/89/144 ms across 770 samples; zero completed duplicate intervals and zero failures; RAF p95/worst 18.6/51.6 ms; heap -3.68 MiB; DOM +12; provider 1 primitive/6 listeners.
- Accessibility: representative keyboard focus contrast 7.87–8.03:1. Effective-200% evidence uses 720×500 CSS at DPR 2, activates the ≤768px media query, contains document/body width to 720px and reaches Replay, Trade and Journal by the correct sequential/roving keyboard path.
- Recovery/provenance: backup/restored SQLite SHA-256 equal, practice/indicator/drawing/journal browser comparisons exact, production DB SHA-256 remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`, HEAD remains `108aa5dc0e26994607836e2b3b33f482e3791b4e`, and protected tag target remains `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Reviewed screenshots: 1440×1000 inspector, 1280×800 compact, effective-200%, focused control, sustained long-history stress and restored workspace. The 50-drawing state is intentionally dense; the 1280 indicator strip retains its accepted internal horizontal overflow.
- Deviations were evidence-driven and bounded: UTC daily-candle WebSocket serialization, acknowledged HTTP manual Next, cancellation-safe console handling, orphaned request cancellation and matched/timing-correct churn measurement. No dependency, schema migration, telemetry, external data, production DB mutation or protected provenance change was introduced.
- Status: DEV closure evidence is green and stops here for independent Reviewer inspection. No release approval, tag or product-complete declaration is made.

## Final closure Reviewer gate — 2026-07-19

- Reviewer accepted B5-R01–B5-R05 product behavior and independently reproduced all technical gates, 265/265 short product UAT, 19/19 negative fixtures and correct computed 720×500 reflow on a temporary DB copy.
- The `05-41-28Z` canonical manifest is not durable because `files.restoredDatabase` points outside the bundle to `RUNTIME_DIR/restored.db`.
- The retained effective-200% screenshot does not render the measured small-viewport composition; it shows the desktop composition clipped at 720px. Machine geometry is accepted, but the claimed reviewed visual evidence is not.
- One evidence/harness-only closure B5-R06–B5-R07 is required. Retain the restored DB inside the artifact root, reject external canonical paths, retain/validate the negative suite, and capture truthful Replay and Journal 720×500 reflow images with internal layout/overflow geometry.
- Product code changes are prohibited. Produce one new full 1,800-second self-contained 277/277 bundle and stop at Reviewer gate. Authority: `docs/dev-prompts/BATCH_5_EVIDENCE_SEALING_PROMPT.md`.

## Evidence sealing execution plan — 2026-07-22

- Scope and acceptance: implement only the evidence/harness closure B5-R06 and B5-R07 authorized by `docs/dev-prompts/BATCH_5_EVIDENCE_SEALING_PROMPT.md`. Preserve the accepted B5-R01–B5-R05 behavior, all 272 returned IDs and the exact five closure IDs; do not modify backend/frontend product code or release state.
- Affected harness files: `scripts/run-batch5-hardening.sh`, `scripts/batch5-manifest.mjs`, `scripts/batch5-evidence-negative-selftest.mjs`, and `scripts/product-uat.mjs`. After every gate is green, update this ExecPlan plus `docs/V3_ACCEPTANCE_MATRIX.md`, `docs/V3_RELEASE_CANDIDATE_NOTES.md`, `docs/V3_VERIFICATION_AND_RECOVERY.md`, and `docs/V3_EVIDENCE_INDEX.md` to point at the sealed bundle.
- B5-R06 method: place the restored database under the canonical artifact root; require every manifest file to resolve to a regular non-symlink file contained by that root; retain and hash `negative-selftest.json`; require its exact 20-case unique passing set; add the existing-external-file escape fixture; and perform a post-service-stop bundle containment/hash verification independent of the temporary runtime directory. The production database remains read-only provenance and is excluded from canonical `manifest.files`.
- B5-R07 method: use Playwright `page.setViewportSize({ width: 720, height: 500 })`, record computed style/bounds/client/scroll/offset geometry for the sidebar, shell, main, workspace, workspace body, chart region and details region, assert the accepted column/stacked reflow and horizontal containment while preserving vertical scroll, and retain separate truthful Replay and keyboard-selected Journal screenshots with recorded dimensions. Restore 1440×1000 before continuing desktop/compact assertions.
- Rollback: revert only the four harness changes and the evidence-ledger/ExecPlan additions from this sealing closure, and discard only the newly generated sealing artifact root. The returned `2026-07-19T05-41-28Z` bundle remains immutable historical evidence; production data and product code are outside rollback scope.
- Exact focused verification commands: `node scripts/batch5-evidence-negative-selftest.mjs`; `SUMI_PRODUCT_UAT_BATCH5_FOCUSED=1 ./scripts/run-product-uat.sh` if supported by the harness, otherwise `./scripts/run-product-uat.sh`; inspect the two 720×500 screenshots and the R04 geometry/result evidence; `git diff --check`.
- Exact final verification commands: backend full pytest; frontend full test/lint/build; `./scripts/verify-v2.sh`; `./scripts/run-product-uat.sh`; `./scripts/verify-product.sh`; `node scripts/batch5-evidence-negative-selftest.mjs`; `caffeinate -dimsu ./scripts/run-batch5-hardening.sh`; then verify manifest containment and all recorded SHA-256 values, exact 277/277 unique identity, production DB SHA-256, `git rev-parse HEAD`, `git rev-parse 'v2.0.0-rc2^{}'`, and `git diff --check`.

## Evidence sealing DEV handoff — 2026-07-22

- Canonical sealed manifest: `test-results/batch5-hardening/2026-07-22T10-06-17Z/manifest.json` (`schemaVersion: 3`, `pass: true`). Product result is 277 passed, 0 failed, 0 blocking failed, 277 unique IDs. Baseline audit preserves all 272 returned IDs and admits exactly R01–R05 as additive blocking checks.
- B5-R06 is sealed: the restored database is retained inside the artifact root at `restore/restored.db` with SHA-256 `c0ca5f106926189f4eed37e451f420d47c50e019910cd7c28b256d661c40c184`; manifest canonical paths are bundle-relative; 12/12 files independently re-open as regular non-symlink files inside the artifact root with matching SHA-256; production `backend/sumi.db` is provenance-only and excluded from `manifest.files`.
- B5-R07 is sealed: the R04 evidence uses real `page.setViewportSize({ width: 720, height: 500 })`, records small-viewport geometry for sidebar/shell/main/workspace/body/chart/details, asserts horizontal containment and vertical stacked reflow, and retains separate truthful Replay and keyboard-selected Journal screenshots parsed as 720×500.
- Sustained scope: 1,800.1 seconds, 134 timestamped actions, 124 full-surface future-boundary samples, 50 drawings, long history at 254 visible candles, 10 matched Analytics/Replay remounts, 11 reloads and meaningful actions in each five-minute window (24/21/22/21/22/20).
- Performance: navigation median/p95/worst 153/166/266 ms; workspace median/worst 576/958 ms at both viewports; indicator median/p95/worst 46/112/186 ms across 772 samples; zero completed duplicate intervals and zero failures; RAF p95/worst 18.5/52 ms; heap -3.91 MiB; DOM +12; provider ownership stable.
- Gates: syntax checks, negative selftest 20/20, focused/standalone UAT checks, backend 99 passed/1 skipped, frontend 129/129, lint/build, `verify-v2.sh`, standalone UAT 265/265 at `2026-07-22T09-14-48-224Z`, `verify-product.sh` UAT 265/265 at `2026-07-22T09-17-28-823Z`, `git diff --check`, canonical hardening and independent manifest/hash/containment audit all passed.
- Provenance unchanged after sealing: production DB SHA-256 `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`; HEAD `108aa5dc0e26994607836e2b3b33f482e3791b4e`; protected tag target `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- Status: DEV evidence sealing is green and stops here for independent Reviewer inspection. No release approval, tag, branch, commit, push, product-complete declaration or production DB mutation is made.

## Sealed evidence final Reviewer approval — 2026-07-22

- Reviewer independently verified the canonical `2026-07-22T10-06-17Z` manifest and closes B5-R06–B5-R07: 12/12 canonical files are bundle-contained regular non-symlink files with matching SHA-256; the restored DB is retained at `restore/restored.db`; and all 20 unique fail-closed negative cases pass.
- The accepted product identity is 277/277 unique passing checks with the 272-ID baseline unchanged and exactly five blocking closure IDs. The accepted sustained scope is 1,800.1 seconds, 134 actions, 124 passing full future-boundary checkpoints, six active windows, 10 route remounts, 11 reloads and 50 drawings.
- Reviewer visually accepted the truthful 720×500 Replay and keyboard-selected Journal captures and corroborated them against the recorded small-viewport component geometry and containment evidence.
- Independent final gates passed: backend 99/1, frontend 129/129, lint, build, negative selftests, `git diff --check` and `verify-product.sh` with a fresh 265/265 result at `test-results/product-uat/2026-07-22T10-54-46-386Z/results.json`.
- Production DB SHA-256, HEAD and protected `v2.0.0-rc2` target remain unchanged. Batch 5 and the V3 acceptance contract are approved and closed. No release mutation was performed; commit, push or tag creation requires separate explicit authority.
