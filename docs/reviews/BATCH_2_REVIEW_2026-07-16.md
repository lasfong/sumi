# Batch 2 Reviewer gate — 2026-07-16

## Verdict

Status: **RETURN TO DEV FOR BOUNDED HARDENING. Batch 2 is not closed and Batch 3 is not authorized.**

The Indicator Manager domain model, persistence, per-instance ownership, backend-authoritative calculation, add/edit/hide/remove flows, duplicate instances, warmup filtering, and core rendering direction are materially stronger than the pre-Batch-2 product. The technical gates pass. However, the browser evidence does not support the claimed PASS for all I-01–I-13.

Reviewer classification:

| Acceptance | Verdict | Reason |
| --- | --- | --- |
| I-01–I-07 | Provisionally accepted | Domain/UI actions and reload state are supported by code, focused tests, and browser state evidence. |
| I-08 | PARTIAL | Pane series and top manager cards exist, but the required title/legend/values/settings/visibility/close chrome is not actually associated with each chart pane. Lifecycle evidence also leaves CCI at an unusable height. |
| I-09 | FAIL | The declared fixed 4:1 policy is not stable after remount/async series creation or at 1280×800. |
| I-10 | Provisionally accepted | MACD line/signal/histogram/zero exist and are visually distinguishable in the wide artifact. |
| I-11–I-12 | PARTIAL | Reference lines are rendered, but their values are not visibly labeled; current UAT proves hidden React text, not what a trader can see. |
| I-13 | Provisionally accepted | Early warmup produces no zero series and no crash. |

## Independent verification

- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 15 files/51 tests; lint/build PASS.
- Independent `./scripts/run-product-uat.sh`: command PASS; result `test-results/product-uat/2026-07-16T15-42-49-030Z/results.json` reports 64 passes, 7 retained drawing failures, `blockingFailed: 0`, and no uncaught runtime errors.
- Production DB SHA-256 after the independent run remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
- Reviewer inspected `01-indicators.png` and `07-compact-1280x800.png` from the independent run.

The command-level green result is not accepted as product closure because the UAT assertions are too weak for the failure visible in their own machine state and screenshots.

## Findings

### B2-R01 — P1 — Fixed 4:1 layout is not deterministic or usable after lifecycle churn

The initial wide snapshot happens to show the intended policy: price 298px and each subpane approximately 73–75px. After ten route mount/unmount cycles, the same UAT records price/Volume/RSI/MACD at 137px and CCI at 40px. After switching to 1280×800 it records price/Volume/RSI/MACD at 91px and CCI at 24px. The compact screenshot confirms the bottom CCI pane is compressed to a sliver.

This is not a 4:1 price-to-subpane allocation and does not meet the accepted fixed-responsive alternative in I-09. `PaneManager.get()` resets only the price factor during asynchronous pane creation, while layout is reconciled opportunistically from per-request completion. The final state depends on creation/completion order.

Required closure:

- make pane creation, ordering, and factor reconciliation deterministic after every add/remove/show/hide/reorder, request completion, reload/remount, and resize;
- prove price remains approximately four times each visible subpane, with subpanes mutually consistent and readable at both required viewports;
- never allow a visible configured pane to collapse to 24/40px without an explicit product policy for overflow or reduced visible panes;
- add focused pane-manager tests covering asynchronous creation order and repeated reconciliation.

### B2-R02 — P1 — Successful backend work is displayed as a request error

The independent compact artifact visibly shows CCI status `error` and “Request failed — retry by showing or editing.” The backend log and machine results show all CCI indicator responses were HTTP 200.

The controller's single promise `catch` covers request execution, semantic mapping, chart series insertion, and pane layout. A chart/layout exception is therefore mislabeled as a network/request failure and does not make `runtime.no-errors` fail. This both harms the user and hides an integration defect behind a recoverable request state.

Required closure:

- separate transport/abort/stale outcomes from semantic-render and chart/layout failures;
- an HTTP 200 must not be reported as “Request failed” because a later chart operation failed;
- render/layout failures must be observable to the Batch 2 gate and must not leave a pane/series partially applied while claiming a healthy chart snapshot;
- UAT must fail if any active indicator card is `error` at the final wide, reload, lifecycle, or compact checkpoints;
- track failed/aborted requests as well as successful responses; counting exactly 40 successes is not proof that no additional request failed.

### B2-R03 — P1 — I-08/I-11/I-12 are proved by hidden declarations, not trader-visible UX

`IndicatorManager` emits hidden `indicator-pane-*`, `rsi-reference-lines`, `cci-reference-lines`, and `macd-components` nodes. UAT reads these hard-coded values to mark pane legends and reference criteria PASS. These nodes do not prove the Lightweight Charts pane actually displays the required chrome or readable reference values.

The chart creates reference price lines with `axisLabelVisible: false`. The screenshots show faint dashed lines, but not readable 30/50/70 or -100/0/100 labels. Settings/hide/remove controls live only in the horizontally scrolling manager strip, not in visible chrome clearly attached to the corresponding pane.

Required closure:

- provide visible, unambiguous pane-associated title/legend/current values/settings/visibility/close affordances for each oscillator/Volume pane, without duplicating state ownership;
- make RSI 30/50/70 and CCI -100/0/100 values visible and attributable to their actual chart reference lines/panes;
- remove hidden hard-coded acceptance surrogates as proof of I-08/I-11/I-12;
- use provider/chart snapshots only to prove actual series/reference configuration, and visible DOM/browser evidence to prove user-facing labels/controls;
- review label/value overlap at 1440×1000 and 1280×800.

### B2-R04 — P1 — Product UAT accepts states that contradict the acceptance criteria

The lifecycle assertion checks key uniqueness and successful-response count but ignores pane factors/heights, active error states, failed requests, and pane ordering. The compact assertion accepts every height greater than zero, so a 24px pane passes. I-09 only checks that the price pane appears first and every subpane is nonzero. I-11/I-12 read hidden strings.

Required closure:

- assert the recorded fixed-layout ratio and minimum readable policy after initial load, reload, each of ten remounts, and compact resize;
- assert document order equals actual non-price pane order at every checkpoint;
- assert every visible active instance reaches `ready` or legitimate `warming`, never `error`, before evidence capture;
- assert visible pane chrome text and controls for the exact stable instance ID;
- assert actual chart snapshot reference labels/values and visible user labels, not hidden declarations;
- retain every existing test ID and all seven Batch 3/legacy drawing failures.

## Scope protection

No accepted I-01–I-07, I-10, or I-13 behavior is reopened except where regression coverage is necessary. Do not redesign the full application, add dependencies, change backend contracts, calculate indicators in the frontend, migrate backend data, or begin Batch 3 drawing tools.

Use `docs/dev-prompts/BATCH_2_REVIEW_HARDENING_PROMPT.md` for the bounded DEV continuation. Reviewer approval is required before Batch 3 planning.

## Final Reviewer closure — 2026-07-17

Status: **APPROVED AND CLOSED. Batch 3 is authorized under its dedicated DEV prompt.**

The Reviewer inspected the B2-R01–B2-R04 implementation, focused tests, strengthened browser assertions, machine state, and both required viewport artifacts. All four returned findings are closed:

- **B2-R01 closed:** panes are materialized in Sumi document order before asynchronous indicator completion and reconciled through official Lightweight Charts v5 pane APIs. The independent run holds approximately 4:1 at initial load, reload, every one of ten remounts, and exact 240/60/60/60/60px at 1280×800.
- **B2-R02 closed:** transport, abort/stale, mapping, and chart/layout failures have separate paths. Series application stages and rolls back before replacing prior ownership; the independent run has no active error state, request failure, page error, console error, or runtime error.
- **B2-R03 closed:** RSI/MACD/CCI/Volume have visible pane-associated, stable-ID chrome with title, parameters, current values, references, Settings, Hide, and Close. Hidden hard-coded acceptance surrogates were removed.
- **B2-R04 closed:** product UAT now checks actual native order, height/stretch ratios, sibling consistency, lifecycle/remount stability, compact geometry, active runtime state, request outcomes, visible pane controls, and real chart reference configuration.

Independent final evidence:

- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend 16 files/57 tests; lint/build PASS.
- `./scripts/run-product-uat.sh`: PASS; independent result `test-results/product-uat/2026-07-17T16-26-19-551Z/results.json` records 79 passes, 7 retained out-of-scope drawing failures, `blockingFailed: 0`, no failed indicator request, and zero runtime errors.
- Independent 1440×1000 and 1280×800 screenshots were reviewed. Required pane chrome is contained and readable; no pane is collapsed and no false request error remains.
- Production DB SHA-256 remains `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

I-01 through I-13 are accepted for the Batch 2 scope. This does not declare Sumi product-complete or professionally ready: the seven retained drawing failures are the authorized Batch 3 scope, and the later integrated trading-practice workflow remains unreviewed.
