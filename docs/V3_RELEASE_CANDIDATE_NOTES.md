# V3 release-candidate acceptance notes

The sealed Batch 5 evidence was independently approved and the V3 acceptance gate closed on 2026-07-22. This approval does not itself create a commit, tag or release.

## Evidence outcome

- Product UAT: 277 passed, 0 failed, 0 blocking failed.
- Preserved baseline: all 272 returned baseline IDs exactly once with unchanged pass values; exactly five additive blocking `batch5.closure.*` IDs.
- Canonical sealed bundle: `test-results/batch5-hardening/2026-07-22T10-06-17Z/manifest.json` (`schemaVersion: 3`, `pass: true`).
- Sustained session: 1,800.1 seconds, 134 timestamped actions, 124 full-surface no-future checkpoints, 10 matched route remounts, 11 reload/resume cycles, 50 persisted drawings and 0 runtime/provider/request failures.
- Performance: workspace usable median 576 ms/worst 958 ms; navigation median 153 ms/p95 166 ms/worst 266 ms; indicator median 46 ms/p95 112 ms/worst 186 ms across 772 samples; RAF p95 18.5 ms/worst 52 ms; five long tasks/worst 52 ms; heap growth -3.91 MiB; DOM growth 12 nodes; zero completed duplicate request intervals.
- Recovery: temporary SQLite online backup semantic checksum equal; restored practice, indicators, drawings and journal compare exactly; restored DB is retained inside the bundle at `restore/restored.db` with SHA-256 `c0ca5f106926189f4eed37e451f420d47c50e019910cd7c28b256d661c40c184`; production DB SHA-256 unchanged.

## Bounded fixes from red evidence

- Added reusable modal focus containment, Escape close and opener restoration; completed PracticeRail ARIA tab/tabpanel and keyboard relationships; added visible focus styling.
- Removed Google Fonts requests so replay traffic remains localhost-only.
- Isolated global replay/drawing shortcuts from interactive/editable/modal/PracticeRail targets while preserving exact-once chart-background actions.
- Fixed timezone-independent WebSocket daily-candle serialization and moved manual Next to the acknowledged HTTP mutation; WebSocket remains the autoplay transport.
- Stopped expected indicator cancellation from polluting console errors and abort orphaned indicator work when an instance changes work key.
- Hardened the runner with full API/chart/indicator/marker/provider future-boundary snapshots, 20 fail-closed negative cases, long-history stress, six-window coverage and matched route-remount/reload measurements.
- Corrected accessibility evidence to use an effective 720×500 CSS viewport, ARIA roving-tab navigation, measured focus contrast, and separate truthful Replay/Journal reflow screenshots instead of CSS zoom/direct-Tab proxies.
- Sealed the evidence manifest so every canonical file is bundle-relative, a regular non-symlink under the artifact root, independently SHA-256 verified after write, and production `backend/sumi.db` remains provenance-only rather than a canonical artifact.

## Reviewer closure

- Reviewer independently verified the acceptance matrix, manifest, `results.json`, recovery evidence, screenshots and final technical gates; no Batch 5 finding remains open.
- A fresh independent `verify-product.sh` run passed with 265/265 at `test-results/product-uat/2026-07-22T10-54-46-386Z/results.json`.
- The prior `2026-07-19T01-05-23Z` 272-ID bundle and returned `2026-07-19T05-41-28Z` closure bundle remain historical evidence; the sealed canonical closure bundle is `2026-07-22T10-06-17Z`.
- Stress-state screenshots are intentionally crowded with 50 drawings. Core controls remain reachable, but this evidence does not claim mobile-first usability.
- The existing `v2.0.0-rc2` label remains visible in historical shell chrome and the protected tag target remains unchanged; no tag was created or moved.
