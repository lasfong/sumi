# Batch 5 — Product hardening and V3 release-candidate evidence — standalone DEV prompt

You are the DEV task for Sumi Batch 5, the final bounded V3 hardening batch. Batch 1–4 are Reviewer-approved. Your job is to produce an evidence-backed V3 release-candidate handoff; you do not self-approve, create a release tag, or call the product finished.

## Mandatory reading before any change

Read completely, in order:

1. `AGENTS.md`
2. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
5. `docs/DEVELOPMENT_OPERATING_MODEL.md`
6. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
7. `PLANS.md`
8. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`
9. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
10. `docs/reviews/BATCH_3_REVIEW_2026-07-18.md`
11. `docs/reviews/BATCH_4_REVIEW_2026-07-18.md`
12. all Batch 1–4 ExecPlans in `docs/exec-plans/`

Inspect the dirty tree and preserve every unrelated user/Reviewer/DEV change. Use the current checkout. Do not create a branch/worktree, stage, commit, push, merge, reset, clean, checkout or tag. Do not move/rewrite `v2.0.0-rc2`. Never run tests against or mutate `backend/sumi.db`; use copied or temporary databases. Keep local-first behavior and transmit no user market/trading/journal data externally.

Before product or harness changes, create `docs/exec-plans/BATCH_5_PRODUCT_HARDENING_V3_RC.md` following `PLANS.md`. Record scope, current 254-ID baseline, affected files, explicit budgets/measurement method, acceptance mapping, rollback, stop conditions and exact commands. Red evidence must precede any defect fix. Do not broaden Batch 5 into new feature development.

## Outcome and acceptance authority

Close the release evidence for every G-01–G-05, R-01–R-05, I-01–I-13, D-01–D-11 and T-01–T-05 criterion without weakening, renaming or deleting accepted assertions. Batch 5 adds hardening/evidence; it must preserve all 254 accepted IDs exactly once and blocking.

### 1. Sustained practice-session UAT

Run and retain a realistic 30-minute uninterrupted practice session through the real UI on deterministic temporary data. Cover replay navigation including keyboard/±5/autoplay/pause/speed, indicators, drawings/edit/history/magnet, BUY/HOLD/SKIP/LIMIT/fill/T+2/CLOSE, journal/checklists, rewind/forward and reload/resume. Record timestamps/duration, exact actions, final machine state, screenshots and all error arrays. Do not substitute a short scripted path for the sustained-session requirement; a deterministic automated scenario may accompany the session but not erase its elapsed-duration evidence.

Prove throughout that future candles never appear in API/chart/indicator/marker/derived state and that there are no duplicate requests, executions, markers, primitives, listeners or stale panels.

### 2. Performance and memory hardening

Define conservative pass/fail budgets before measurement, based on the supported 1440×1000 and 1280×800 workstation rather than one lucky sample. Measure repeatably with long supported histories, the documented maximum/representative indicator set, many persisted drawings, navigation bursts, autoplay, pan/zoom, repeated route remounts and reload.

Retain machine-readable measurements for initial render, navigation responsiveness, indicator request count/latency, frame/long-task behavior where available, heap/listener/primitive/series growth and cleanup after remount. Use warmup plus multiple samples and report median/worst case. Fix only demonstrated P0/P1 regressions within existing architecture. If a budget requires provider/chart/architecture replacement, stop for Reviewer.

### 3. Migration, backup and restore

Using temporary copies only, prove:

- a fresh database migrates to head and supports the complete workflow;
- a copy of the current production-format database migrates/opens without modifying the original;
- documented backup and restore preserve replay sessions, decisions/orders/executions/trades/journal, indicator documents and drawing documents with checksums/counts/semantic equality;
- malformed or legacy indicator/drawing/journal data follows the already approved quarantine/compatibility behavior without silent loss;
- rollback instructions identify code/data compatibility and the protected `v2.0.0-rc2` boundary.

Do not invent an unsafe in-place migration test or claim disaster recovery without a verified restored copy.

### 4. Accessibility and keyboard pass

Audit the core workflow at both supported viewports using real keyboard interaction. Verify focus order/visibility, tab/tabpanel/dialog semantics, labels/names, modal focus containment and return, Escape behavior, editable-control shortcut isolation, drawing delete/history shortcuts, replay shortcuts outside editors, disabled/historical/mobile-limited states, alerts/status feedback, and usable contrast/zoom for core information.

Use automated accessibility inspection if already available or narrowly justified; do not add a dependency without license and necessity review in the ExecPlan. Add focused and browser assertions for every fixed defect. Record accepted non-blocking limitations explicitly; unresolved core P0/P1 accessibility issues block the batch.

### 5. Release documentation and evidence ledger

Reconcile and clean the V3 documentation without rewriting historical evidence. Produce at least:

- a final acceptance matrix mapping every G/R/I/D/T ID to test IDs and artifact paths;
- V3 release-candidate notes describing user-visible capabilities, upgrade/backup/restore, minimum width, local-first guarantee, known limitations and rollback;
- exact verification instructions reproducible from a clean temporary runtime;
- an evidence index for sustained UAT, performance/memory, migration/restore, accessibility, wide/compact screenshots and machine results.

Do not use “TradingView-like.” Do not use “professional manual replay and TA practice product,” “release-ready,” or “complete” in the DEV handoff; the Reviewer decides whether the release-language gate is satisfied.

## Harness and change rules

- Preserve all 254 Batch 1–4 IDs, names, pass values and blocking semantics. New Batch 5 assertions must be additive, uniquely named `batch5.*`, blocking where they represent release criteria, and must fail honestly.
- Browser evidence must use temporary database/services and retain results even on failure. Required viewports remain 1440×1000 and 1280×800; below 1180px stays explicitly limited.
- No acceptance weakening, hidden failure filtering, arbitrary sleeps as correctness, telemetry, external data transmission, chart/drawing dependency, backend indicator duplication, provider raw JSON authority, private chart API, migration of `backend/sumi.db`, or unrelated cleanup.
- If hardening exposes a product defect, add a red focused/browser reproduction, implement the narrow fix, rerun the full gates and document the deviation. Stop if resolution requires a new product capability or architecture decision.

## Required final verification

At minimum run and retain:

```bash
git diff --check
cd backend && ../.venv/bin/python -m pytest -q
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
git rev-parse HEAD
git rev-parse 'v2.0.0-rc2^{}'
```

Also run the sustained-session, performance/memory, migration/backup/restore and accessibility commands recorded in the ExecPlan. Compare final product-UAT IDs with the accepted baseline `test-results/product-uat/2026-07-18T15-23-28-222Z/results.json`, reporting missing, duplicate, renamed, changed-pass, changed-blocking and additive IDs.

Review all required screenshots manually. Final handoff must include exact commands/counts, budgets and measured values, artifact paths, error arrays, acceptance matrix, known limitations, DB hash, HEAD/tag and dirty-scope audit. Update the ExecPlan progress/decisions/deviations/evidence. Stop at Reviewer gate without self-approval, git mutation, release tagging or post-V3 work.
