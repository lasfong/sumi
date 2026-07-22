# Batch 5 Reviewer gate — 2026-07-19

## Verdict

Status: **RETURNED FOR ONE BOUNDED RELEASE-EVIDENCE CLOSURE. V3 release approval and release tagging remain unauthorized.**

The Batch 5 bundle is substantial: it retains a real 1,800-second browser session, fixed performance budgets, 50 drawings, temporary-database backup/restore, local-only traffic, additive accessibility work and a complete documentation set. Existing technical gates are green. The release-candidate gate cannot close because the Reviewer found one reproducible keyboard integrity bug and four places where a passing Batch 5 assertion or manifest is weaker than the contract it claims.

## Independent evidence accepted at this gate

- Canonical DEV result contains 272 unique passing IDs, zero blocking failures, empty runtime/provider/indicator-request failure arrays, all 254 Batch 1–4 IDs unchanged and 18 additive `batch5.*` IDs.
- The retained timeline spans 1,800.143 seconds with 119 entries and a maximum gap of about 54.5 seconds. Each five-minute window contains repeated real UI actions. The 115 no-future samples are timestamped and individually green under their current predicate.
- Submitted performance measurements are internally consistent and well inside the recorded numerical budgets: navigation p95 164 ms, indicator p95 55 ms, RAF p95 17.9 ms, two 50 ms long tasks, negative matched heap growth, 12-node DOM growth, one primitive and six listeners.
- SQLite online backup source/backup counts and semantic SHA-256 match. Restored browser practice/indicator/drawing/journal comparisons are true with no restore runtime error. Production-format copy migration evidence exists and the production database hash is unchanged.
- Reviewer inspected the 1440×1000, 1280×800, 200% and 50-drawing stress artifacts. Standard workstations remain usable; the stress screenshot is intentionally dense.
- Independent direct gates: backend 97 passed / 1 skipped; frontend 20 files / 116 tests; lint, production build, `git diff --check` and `./scripts/verify-v2.sh` passed.
- Production DB SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`. HEAD remained `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2^{}` remained `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.

These green facts are retained. They do not override the findings below.

## Findings

### B5-R01 — P1 — PracticeRail keyboard navigation also advances Replay

The global Replay key handler excludes only `INPUT`, `TEXTAREA` and `SELECT`. It still processes keys originating from buttons, tabs, dialogs, links and contenteditable controls. `PracticeRail` handles ArrowRight/ArrowLeft with `preventDefault()` but not propagation, so the same event reaches the window handler and invokes Replay Next/Prev.

The submitted sustained timeline itself proves the defect. Iteration 4 ends at index 70 after autoplay; iteration 5 is recorded only as `keyboard-tab-navigation` but its no-future sample is index 71. The pattern repeats at iteration 15, moving 72 to 73. The passing tab assertion checks only that Trade changes to Journal; it never asserts the replay index stayed unchanged. Space on a focused button has the same double-action risk.

This violates R-03, the Batch 4 keyboard-isolation contract and the Batch 5 accessibility budget.

Required closure:

- centralize a target/default-prevented guard so global replay/drawing shortcuts run only from the non-interactive workspace, not from button/link/tab/dialog/contenteditable/form controls;
- use component propagation guards where appropriate, but do not rely on scattered exceptions as the only policy;
- prove Arrow/Home/End on PracticeRail changes tabs without changing replay index, Space/Enter on core buttons invokes only the button, dialog/editable keys never navigate/delete drawings, and shortcuts still work from the chart/workspace background;
- add focused tests and exact blocking browser evidence for unchanged replay/drawing state on every isolated path.

### B5-R02 — P1 — Continuous no-future assertion checks only API candle count and markers

The recorded invariant says every checkpoint compares API candles, chart-visible input, marker/indicator/drawing inputs and `current_index`. `verifyNoFuture()` actually compares practice index, session index, API candle length and marker dates only. It does not inspect chart candle count/max date, active indicator response/input dates, or visible provider/drawing input dates. Therefore `batch5.sustained.no-future-every-checkpoint` can pass while three named downstream surfaces are stale or future-leaking.

Required closure:

- at every sustained checkpoint, record and compare authoritative index/date with API candle count/max date, chart input count/max date, every active indicator response/input max date, marker max date and every visible provider drawing input/coordinate date;
- distinguish retained future drawing documents from what the provider is allowed to render after rewind; assert the visible provider state, not an inappropriate blanket rejection of canonical history;
- retain fail-fast partial/live evidence containing the first divergent surface and exact values;
- add a controlled negative harness test proving each surface-specific mismatch makes the checkpoint and final run fail.

### B5-R03 — P1 — Canonical manifest is fail-open

`batch5-manifest.mjs` sets `manifest.pass` from database semantic equality, restore pass, baseline-audit pass and production DB hash equality. It never reads or requires the referenced 272-ID product result to have zero failed/blocking checks and empty error arrays. `batch5-evidence-audit.mjs` also does not require additive checks to pass. Consequently a hardening run with a failed `batch5.*` assertion can still produce `manifest.pass: true`.

The manifest also omits the pre-migration application-schema verdict and does not make the canonical bundle self-validating against all claimed provenance/recovery files.

Required closure:

- make the manifest read and checksum every canonical retained JSON/file it cites;
- require product result count/uniqueness, every check passing, zero failed/blocking, expected error-array policy, baseline preservation and every additive Batch 5 check passing/blocking;
- require fresh/copy migration verdicts, pre-copy application-schema pass, backup/restored semantic equality, restored browser equality, production DB hash and exact HEAD/tag targets;
- make the evidence audit fail on a deliberately failed additive check, duplicate/removed/changed baseline check, missing artifact, failed restore/migration and changed DB/provenance; retain negative self-tests.

### B5-R04 — P1 — Focus and 200% assertions do not prove their recorded accessibility budgets

The visible-focus check passes any outline at least 2px. Its own evidence is a 3px black outline with no comparison to the unfocused style or adjacent dark background, so it does not prove a visible focus change/contrast. The 200% assertion checks only Playwright `isVisible()` for three controls. Its evidence reports `scrollWidth: 2880` with `clientWidth: 1440`; `isVisible()` does not establish that the controls are in the viewport, keyboard reachable without context loss, or free from application-wide clipping.

Required closure:

- compare focused and unfocused computed styles and verify a visibly contrasting focus indicator against the actual adjacent background; retain a focused-control screenshot;
- at 200%, use bounding boxes, viewport/scroll geometry and real Tab/activation to prove Replay, Trade and Journal are reachable and usable without hidden core controls or two-dimensional context loss;
- if the existing 1180px minimum and 200% requirement cannot coexist without a responsive product change, stop for Reviewer with exact geometry instead of weakening the assertion or silently accepting the conflict;
- keep manual composited-contrast limitations explicit and do not extrapolate opaque-control sampling to the whole UI.

### B5-R05 — P1 — Sustained/performance evidence does not exercise the full recorded stress scope

The 30-minute loop is meaningful but narrow: after creating 50 Horizontal drawings in one burst, it repeats navigation around indices 69–96, autoplay, tab movement, indicator hide/show, magnet selection, wheel input and reload. It does not timestamp the earlier BUY/LIMIT/T+2/CLOSE/journal/drawing edit/history actions, does not exercise long visible history, and does not measure heap/DOM/provider/indicator ownership across the required route remount churn. The current performance values therefore describe a short visible history near 70–96 bars rather than the recorded long-history workload.

Required closure:

- timestamp the complete real practice workflow, including trade lifecycle, journal/checklist, all drawing tools/edit/history/magnet and indicator lifecycle, not only the repeated tail loop;
- distribute meaningful actions throughout all six five-minute windows and report action-category/window coverage mechanically;
- advance to a genuinely long visible supported history before the stress/performance segment and record visible counts;
- include ten analytics/replay route remounts plus reload cleanup inside matched heap/DOM/provider/listener/indicator/request measurements;
- rerun the full 1,800-second canonical session after closure; a short smoke may develop the harness but cannot replace final evidence.

## Scope protection

Keep the approved Batch 1–4 architecture, current chart/provider/IndicatorEngine boundaries, Sumi documents, temporary database, local-only behavior and all accepted IDs. Fix only the keyboard collision and evidence/harness defects above. Do not add new product features, dependencies, telemetry, cloud backup, migrations, chart/provider replacement, production DB writes, release tag, commit or push.

Use `docs/dev-prompts/BATCH_5_REVIEW_CLOSURE_PROMPT.md` as the only DEV authority. Preserve all 272 canonical IDs and add exact blocking closure IDs. Stop again at Reviewer gate.

## R04 stop review addendum — 2026-07-19

Status: **BOUNDED CONTINUATION AUTHORIZED. A RESPONSIVE REDESIGN IS NOT AUTHORIZED. V3 release approval remains unauthorized.**

The DEV task correctly stopped on the R04 condition and retained `test-results/product-uat/2026-07-19T03-05-48-899Z/results.json`. The focused result contains 262 passing and three failing checks. `batch5.closure.R01.keyboard-isolation` is accepted as focused green evidence. R02, R03 and R05 have implementation in the sustained harness but still require the final canonical run. Reviewer independently ran `node scripts/batch5-evidence-negative-selftest.mjs`; all 19 audit, manifest and future-boundary negative cases returned the required failure verdict.

The submitted R04 result does not establish that the product requires a responsive redesign:

- the harness sets `document.documentElement.style.zoom = '2'`. This doubles the rendered 1440×1000 layout while retaining the 1440 CSS-pixel media-query viewport, so the existing `@media (max-width: 768px)` reflow is not exercised. It is not an honest browser-zoom reflow test;
- Journal is intentionally `tabIndex=-1` when Trade is the selected tab under the ARIA roving-tabindex pattern. Repeating Tab 120 times cannot be used to prove Journal is unreachable. The valid path is sequential Tab to the selected tab, then ArrowRight to focus and select Journal;
- the failing result was written at 10:06:47 +0700, while the current focus-rule file was modified at 10:07:46 +0700. Therefore the black-outline result is red evidence, not green evidence for the current CSS.

Required continuation:

- remove inline CSS zoom from the R04 assertion. Exercise the 1440×1000 physical reference as an effective 720×500 CSS viewport, and record `innerWidth`, `innerHeight` and `matchMedia('(max-width: 768px)').matches`. A high-DPI context may retain a 1440×1000 physical screenshot, but device-pixel ratio must not be presented as the zoom mechanism;
- reach the selected Trade tab through the sequential keyboard order, use the tablist ArrowRight contract to reach Journal, and prove focus, selection, panel activation and post-scroll viewport intersection. Prove Replay access separately. Vertical reflow/scroll is allowed; application-wide horizontal clipping is not;
- compare unfocused and keyboard-focused computed styles on representative Replay, tab and dialog/opener controls. Parse transparent backgrounds as alpha zero, require at least 3:1 focus-indicator contrast against the adjacent rendered background, and retain the focused screenshot;
- preserve the existing 1180px full-workstation threshold. Below it, the limited-workstation explanation and disabled trading actions remain accepted behavior; the Trade and Journal tabs/panels must still be keyboard reachable;
- if the corrected 720×500 test exposes a real containment defect, only a narrow CSS/layout correction inside the existing small-viewport breakpoint is authorized. Do not change application architecture, navigation, chart/provider boundaries, desktop layouts, acceptance criteria or dependencies. Stop again if more is required.

After a green focused R04 run, execute the full gates and one new canonical run of at least 1,800 real seconds. The canonical result must preserve the 272 baseline IDs exactly once and add exactly the five blocking `batch5.closure.R01`–`R05` IDs, for 277 unique passing checks. Repoint the four V3 evidence documents only after the fail-closed manifest is green. Use `docs/dev-prompts/BATCH_5_R04_CONTINUATION_PROMPT.md` as the sole DEV authority from this point.

## Final closure Reviewer gate — 2026-07-19

Status: **RETURNED FOR ONE EVIDENCE-SEALING CLOSURE. PRODUCT BEHAVIOR B5-R01–B5-R05 IS ACCEPTED. V3 release approval and tagging remain unauthorized.**

The `2026-07-19T05-41-28Z` run establishes the requested product and sustained behavior:

- 277 unique passing checks, zero failures and zero blocking failures; all 272 baseline IDs are unchanged and exactly the five blocking B5-R01–B5-R05 closure IDs are additive;
- 1,800.139 real seconds, 134 timestamped actions, 124 complete future-boundary checkpoints, meaningful activity in all six five-minute windows, 10 Analytics/Replay remounts and 11 reloads;
- green focus contrast evidence on three representative controls, correct ARIA roving-tab navigation and machine evidence for an effective 720×500 CSS viewport;
- 19/19 fail-closed negative fixtures, exact backup/restore semantic equality, exact browser practice/indicator/drawing/journal restore, empty runtime/provider/request-failure arrays and unchanged production DB/provenance.

Reviewer independently reran backend 99 pass/1 skip, frontend 129/129, lint, build, `verify-v2.sh`, `verify-product.sh` with 265/265 product UAT, `git diff --check` and all 19 negative fixtures. Reviewer also ran a Chrome diagnostic against a temporary database copy at 720×500/DPR 2. Computed layout was correct: the media query matched, `.app-shell` used column flow, sidebar width was 704px, workspace/chart/rail width was 696px, chart and rail stacked vertically, and document/body/main/workspace had no horizontal overflow. Product reflow is therefore accepted.

Two release-evidence defects remain.

### B5-R06 — P1 — canonical manifest depends on a non-bundle temporary database

`manifest.files.restoredDatabase.path` is `/var/folders/.../sumi-batch5.HPltEi/restored.db`. Its hash currently matches, but the file is outside `test-results/batch5-hardening/2026-07-19T05-41-28Z/` and will disappear when the operating system cleans the runtime directory. The canonical manifest is therefore not a durable, self-contained evidence bundle. The missing-artifact negative fixture does not test an existing cited file outside the artifact root.

Required closure:

- retain the restored SQLite file inside the canonical artifact directory and point the manifest only to that retained copy;
- make manifest generation reject canonical `files.*` paths outside the artifact root and add an existing-external-file negative fixture;
- include and require the retained 19-case negative-selftest result in the manifest;
- prove every manifest file exists under the final bundle root and matches its SHA-256 after services and runtime-directory use have ended.

### B5-R07 — P1 — retained effective-200% screenshot does not show the measured reflow

The machine evidence and independent diagnostic prove that reflow works. The retained `22-batch5-zoom-200.png`, however, is a 720×500 image of the desktop composition clipped at the right edge: it shows the 260px desktop sidebar beside the chart and does not show the stacked small-viewport sidebar/workspace/rail. This is caused by capturing through the CDP device-metrics override on the existing 1440×1000 Playwright context. The evidence index calls this image reviewed proof of the 720×500 reflow, which it is not.

Required closure:

- capture the effective viewport through a rendering path whose pixels match the computed 720×500 layout, preferably a real `page.setViewportSize({ width: 720, height: 500 })` layout viewport rather than the current CDP screenshot path;
- retain one viewport image showing the reflowed Replay area and a second image after the valid Trade → ArrowRight → Journal keyboard path showing the active, focused Journal tab/panel;
- record `.sidebar`, `.app-shell`, `.app-main`, workspace, chart and rail computed layout plus client/scroll widths, not only document/body geometry;
- visually inspect the new images and remove the unsupported reviewed-image claim for the returned artifact.

This closure is evidence/harness-only. Do not change product code or accepted behavior. Because the runner and canonical evidence contents change, produce one fresh full 1,800-second bundle with the same 277/277 identity and rerun the final gates. Retain `05-41-28Z` as returned historical evidence. Use `docs/dev-prompts/BATCH_5_EVIDENCE_SEALING_PROMPT.md` as the only DEV authority and stop again at Reviewer gate.

## Sealed evidence final Reviewer approval — 2026-07-22

Status: **APPROVED AND CLOSED. BATCH 5 AND THE V3 ACCEPTANCE CONTRACT PASS.**

The Reviewer independently inspected the sealed `2026-07-22T10-06-17Z` bundle and closes B5-R06 and B5-R07:

- the canonical product result contains 277 unique passing checks, zero failures and zero blocking failures; all 272 returned baseline IDs are preserved unchanged and exactly five blocking B5-R01–B5-R05 IDs are additive;
- all 12 manifest files are bundle-relative, exist inside the artifact root as regular non-symlink files and match their recorded SHA-256; the retained restored database is `restore/restored.db` with SHA-256 `c0ca5f106926189f4eed37e451f420d47c50e019910cd7c28b256d661c40c184`;
- all 20 unique fail-closed negative fixtures pass, including the existing external-artifact rejection;
- the 1,800.1-second run records 134 actions, 124 passing full future-boundary checkpoints, meaningful activity in all six five-minute windows, 10 route remounts, 11 reloads and 50 drawings without runtime, provider or request failures;
- the retained Replay and keyboard-selected Journal images are truthful 720×500 captures of the measured reflow. Recorded sidebar, shell, main, workspace, chart and rail geometry confirms the accepted vertical stack and no application-wide horizontal overflow;
- backup/restore semantic equality and exact browser practice, indicator, drawing and journal restoration pass.

Independent final verification passed backend 99/1, frontend 129/129, lint, production build, negative selftests, `git diff --check` and `verify-product.sh`; the latter retained a fresh 265/265 result at `test-results/product-uat/2026-07-22T10-54-46-386Z/results.json`. Production DB SHA-256 remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`, HEAD remains `108aa5dc0e26994607836e2b3b33f482e3791b4e`, and `v2.0.0-rc2^{}` remains `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.

No Batch 5 finding remains open and no further product batch is required by the V3 roadmap. This Reviewer approval closes the acceptance gate; it does not itself authorize or perform staging, commit, push, tag creation or other release mutation.
