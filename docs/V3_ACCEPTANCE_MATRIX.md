# V3 acceptance evidence matrix

Status: independently reviewed, approved and closed on 2026-07-22. This acceptance decision does not itself create a commit, tag or release.

Canonical sealed evidence is `test-results/batch5-hardening/2026-07-22T10-06-17Z/manifest.json` and its product-UAT `results.json` (`277/277`, `0` blocking failures). All 272 returned Batch 5 baseline IDs are present exactly once and unchanged; exactly five blocking `batch5.closure.*` IDs are additive. The returned `2026-07-19T05-41-28Z` bundle is retained as historical evidence, not the sealed canonical bundle.

| ID | DEV evidence status | Principal evidence |
| --- | --- | --- |
| G-01 | Pass | Backend 99 pass/1 skip; frontend 129 pass; lint/build, `verify-v2`, standalone UAT, `verify-product` and canonical hardening pass; final gates listed in the evidence index. |
| G-02 | Pass | Runner provisions fresh/copied temporary SQLite; production DB SHA-256 unchanged. |
| G-03 | Pass | Product results, live evidence, screenshots, recovery JSON, negative selftest and self-contained manifest retained. |
| G-04 | Pass for demonstrated scope | Keyboard-visible focus contrast is 7.87–8.03:1 on representative core controls; corrected 720×500 effective-200% keyboard/reflow evidence is green with truthful Replay and Journal screenshots. Reviewer accepted the demonstrated scope. |
| G-05 | Pass | `batch5.privacy.loopback-only`; sole origin `http://127.0.0.1:15173`; external Google font request removed. |
| R-01 | Pass | 124 sustained checkpoints agree on authoritative/API/chart/indicator/marker/visible-provider boundaries; all 8 negative future-surface fixtures fail closed. |
| R-02 | Pass | Accepted header/OHLCV context checks and reviewed workstation screenshots. |
| R-03 | Pass | 47 measured navigation samples; global shortcut isolation for tabs/buttons/dialogs/forms/contenteditable/inspector and exact-once chart-background shortcuts. |
| R-04 | Pass | Accepted integrated lifecycle IDs; 772 indicator timing samples, zero completed duplicate intervals and zero request failures. |
| R-05 | Pass | 10 matched Analytics/Replay remounts, 11 reload/resume cycles and exact restored workspace comparison. |
| I-01 | Pass | Accepted `batch2.I-01` and active list browser evidence. |
| I-02 | Pass | Accepted add/search/parameter flow checks. |
| I-03 | Pass | Accepted individual remove checks. |
| I-04 | Pass | Accepted hide/show preservation plus sustained hide/show actions. |
| I-05 | Pass | Accepted registry validation/settings checks. |
| I-06 | Pass | Accepted duplicate-type independent instance checks. |
| I-07 | Pass | Reload cycles and exact workspace import/restore comparison. |
| I-08 | Pass | Accepted pane chrome/legend/value/control geometry checks. |
| I-09 | Pass | Accepted fixed responsive 4:1 pane layout at both viewports. |
| I-10 | Pass | Accepted MACD semantic/render/reference checks. |
| I-11 | Pass | Accepted RSI scale and 30/50/70 checks. |
| I-12 | Pass | Accepted CCI -100/0/100 checks. |
| I-13 | Pass | Accepted warming/null and error-boundary checks. |
| D-01 | Pass | Accepted all-tool toolbar labels and creation checks. |
| D-02 | Pass | Accepted active/cancel/tool-switch checks. |
| D-03 | Pass | Accepted provider hit/handle/bounds checks. |
| D-04 | Pass | Accepted body/anchor/inspector edit checks. |
| D-05 | Pass | Accepted UI/keyboard delete and clear/undo checks. |
| D-06 | Pass | Accepted create/move/edit/delete undo/redo checks. |
| D-07 | Pass | Accepted pan/zoom/replay/reload attachment checks. |
| D-08 | Pass | Sumi schema-v1 persistence; exact local/backend and restored workspace equality. |
| D-09 | Pass | Accepted Fibonacci direction/levels/labels/edit checks. |
| D-10 | Pass | Accepted magnet off/OHLC/threshold/visible-only checks and sustained cycling. |
| D-11 | Pass | One primitive/six listeners, zero provider errors and stable churn ownership. |
| T-01 | Pass | Accepted 1440×1000 and 1280×800 workstation geometry; reviewed screenshots. |
| T-02 | Pass | Accepted BUY/HOLD/SKIP/LIMIT/fill/T+2/CLOSE lifecycle checks. |
| T-03 | Pass | Accepted journal/checklist context and keyboard-accessible rail/dialog checks. |
| T-04 | Pass | Accepted marker/position rewind-forward-reload equality and 124 synchronized full-surface checkpoints. |
| T-05 | Pass | 1,800.1-second uninterrupted real-UI run, 134 timestamped actions, all six five-minute windows active, 0 runtime errors. |

## Manual-only limitations

- Contrast inspection is dependency-free and targets representative core controls; it is not a whole-application WCAG audit.
- The 200% check uses an effective 720×500 CSS layout viewport, proves the ≤768px media query, horizontal containment and keyboard access to Replay/Trade/Journal; it is not a claim of mobile-first support.
- Stress screenshots intentionally contain 50 drawings and are visually dense. That is many-drawing evidence, not a recommended everyday layout.
