# Batch 5 final evidence sealing — standalone DEV prompt

You are the DEV task for one final evidence/harness-only closure. Product behavior B5-R01–B5-R05 has been accepted, but V3 release approval and tagging remain unauthorized. Read completely in repository-required order: `AGENTS.md`, its six canonical sources, `PLANS.md`, the Batch 5 initial and closure prompts, `docs/exec-plans/BATCH_5_PRODUCT_HARDENING_V3_RC.md`, and the full `docs/reviews/BATCH_5_REVIEW_2026-07-19.md`. The final closure Reviewer gate in that review controls this task.

Preserve the shared dirty checkout. Do not create or switch a branch/worktree; stage, commit, push, merge, reset, clean, checkout or tag; mutate `backend/sumi.db`; add a dependency; transmit data externally; weaken acceptance; or begin post-V3 work. This closure may change only Batch 5 harness/evidence scripts, tests and documentation. Do not change backend or frontend product code. Append scope, affected harness files, rollback and exact commands to the ExecPlan before implementation.

## B5-R06 — self-contained canonical bundle

1. Store the restored database under the final artifact root, for example `restore/restored.db`, rather than under `RUNTIME_DIR`.
2. Make every entry in `manifest.files` resolve to a regular file inside the canonical artifact root. Reject path traversal, symlink escape and any existing external path. The production database remains a read-only provenance input and must not become a copied canonical product-data artifact.
3. Add the retained `negative-selftest.json` to the manifest, checksum it, parse it and require `pass: true`, the exact expected unique case set and every case passing. After the new external-path fixture below, the final expected count is 20.
4. Add a negative fixture that points a cited canonical artifact at an existing file outside the fixture artifact root; manifest generation must exit nonzero. Keep all existing 19 cases, so the final suite will contain 20 cases unless the Reviewer-approved external-path case is incorporated without removing coverage.
5. After the runner stops restored services, verify every `manifest.files.*.path` is still present under the bundle root and its SHA-256 matches. The bundle must remain verifiable without relying on `RUNTIME_DIR`.

## B5-R07 — honest effective-200% visual evidence

1. Do not use the current CDP device-metrics screenshot as visual proof. Drive a real 720×500 layout viewport whose rendered pixels reflect the active ≤768px media query. `page.setViewportSize({ width: 720, height: 500 })` is acceptable. DPR 2 is optional and must not be described as the zoom mechanism.
2. Extend R04 geometry with computed style, bounding rectangle, client width, scroll width and scroll offsets for `.sidebar`, `.app-shell`, `.app-main`, `.replay-workspace`, `.replay-workspace-body`, `.replay-chart-region` and `.replay-details-region`.
3. Require column shell/reflow, static near-full-width sidebar, vertically stacked chart/rail, and no horizontal overflow in document, body, main or workspace. Preserve vertical scrolling.
4. Retain two truthful 720×500 viewport screenshots: one showing the reflowed Replay area and one after sequential Tab to Trade followed by ArrowRight to the focused/selected Journal tab with its active panel brought into view. Record their paths and pixel dimensions.
5. Restore 1440×1000 afterward and preserve all accepted 1440×1000 and 1280×800 assertions. No product CSS/layout change is authorized; stop if the evidence cannot be produced from the accepted product.

## Verification and final handoff

Run focused harness checks first, including the now-20-case fail-closed negative suite. Then run backend 99/1, frontend 129/129, lint, build, `git diff --check`, `verify-v2.sh`, standalone product UAT and `verify-product.sh`. Finally run one complete `./scripts/run-batch5-hardening.sh` session of at least 1,800 real seconds.

The final canonical product result must remain exactly 277 unique passing checks: the unchanged 272 baseline IDs plus exactly five blocking B5-R01–B5-R05 closure IDs. Require zero failed/blocking checks, unchanged error policy, all future-boundary/category/churn/recovery budgets, a fail-closed self-contained manifest, unchanged production DB SHA-256, exact HEAD/tag targets and no unrelated diff.

Update the four V3 evidence documents to the new bundle only after all checks are green. Retain `test-results/batch5-hardening/2026-07-19T05-41-28Z/` as returned historical evidence and do not call its effective-200% screenshot reviewed reflow proof. Report every manifest path containment result, the retained restored DB hash, all negative cases, both new screenshot dimensions, exact gate counts and provenance. Do not self-approve or use release-approved/product-complete language. Stop at Reviewer gate.
