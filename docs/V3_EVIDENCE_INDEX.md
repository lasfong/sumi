# V3 evidence index

Primary sealed Batch 5 closure evidence root: `test-results/batch5-hardening/2026-07-22T10-06-17Z/`.

Reviewer status: independently approved and closed on 2026-07-22.

| Evidence | Location |
| --- | --- |
| Consolidated manifest | `manifest.json` |
| Full 277-ID product result | `product-uat/2026-07-22T10-06-20-225Z/results.json` |
| Sustained checkpoint stream | `product-uat/2026-07-22T10-06-20-225Z/batch5-live.json` |
| Browser workspace export | `product-uat/2026-07-22T10-06-20-225Z/workspace-export.json` |
| Baseline preservation audit | `baseline-audit.json` |
| Fail-closed negative selftests (20/20) | `negative-selftest.json` |
| Database backup semantic audit | `database-recovery.json` |
| Production-format copy snapshots | `production-copy-pre-snapshot.json`, `production-copy-snapshot.json` |
| Restored database file | `restore/restored.db` |
| Restored database snapshot | `restored-snapshot.json` |
| Restored browser semantic comparison | `restore-results.json`, `restored-workspace.png` |
| Runtime logs | `backend.log`, `frontend.log`, `hardening.log`, `restore.log` |
| Reviewed 1440×1000 screenshot | `product-uat/2026-07-22T10-06-20-225Z/15-two-anchor-inspector-1440x1000.png` |
| Reviewed 1280×800 screenshot | `product-uat/2026-07-22T10-06-20-225Z/20-batch4-compact-1280x800.png` |
| Reviewed 720×500 Replay reflow screenshot | `product-uat/2026-07-22T10-06-20-225Z/22a-batch5-reflow-replay-720x500.png` |
| Reviewed 720×500 Journal reflow screenshot | `product-uat/2026-07-22T10-06-20-225Z/22b-batch5-reflow-journal-720x500.png` |
| Reviewed focused-control screenshot | `product-uat/2026-07-22T10-06-20-225Z/23-batch5-focused-control.png` |
| Reviewed sustained stress screenshot | `product-uat/2026-07-22T10-06-20-225Z/batch5-sustained-100.png` |

Screenshot review: the standard and compact workstations retain chart, indicator panes, replay controls and practice rail; inspector controls remain contained. The sustained image proves the intentional 50-drawing/long-history stress state and is dense by design. At 1280 the indicator strip uses its accepted internal horizontal overflow. The 720×500 reflow evidence activates the ≤768px layout, has no application-wide horizontal overflow, stacks chart/details vertically with preserved vertical scroll, and reaches Replay/Trade/Journal by the recorded keyboard path. The two retained reflow screenshots are parsed as exactly 720×500. Representative focus contrast measures 7.87–8.03:1.

Earlier failed/smoke runs are diagnostic evidence, not the canonical sealed pass bundle. The final manifest identifies the canonical run, verifies 12/12 canonical files as bundle-contained regular non-symlink files with matching SHA-256, includes the 20/20 negative selftest, and confirms the production database hash and protected provenance. Production `backend/sumi.db` is provenance-only and excluded from canonical `manifest.files`.

Post-hardening final gate evidence:

- Standalone `run-product-uat.sh`: `test-results/product-uat/2026-07-22T09-14-48-224Z/results.json` — 265/265.
- `verify-product.sh` product UAT: `test-results/product-uat/2026-07-22T09-17-28-823Z/results.json` — 265/265; the command also reran backend 99 pass/1 skip, frontend 129 pass, lint, build and fresh migration.
- Reviewer independent `verify-product.sh`: `test-results/product-uat/2026-07-22T10-54-46-386Z/results.json` — 265/265 with zero failed or blocking checks; backend 99 pass/1 skip, frontend 129 pass, lint, build and fresh migration also passed.
- Historical returned bundle: `test-results/batch5-hardening/2026-07-19T01-05-23Z/` — 272/272 before the five-finding review closure.
- Historical returned closure bundle: `test-results/batch5-hardening/2026-07-19T05-41-28Z/` — 277/277, returned for evidence sealing because the restored DB path escaped the bundle and the single zoom screenshot was not truthful visual reflow proof.
