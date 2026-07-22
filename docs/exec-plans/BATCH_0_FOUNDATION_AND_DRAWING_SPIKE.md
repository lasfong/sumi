# Batch 0 — Foundation validation and drawing-provider spike

## Outcome

Produce an evidence-backed drawing-provider decision and an approved target component/state design, using the deterministic product harness as the authority. Do not implement the V3 Indicator Manager or rebuild Replay UI in this batch.

## Context and problem

- Current review: `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`.
- Architecture direction: `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`.
- Product plan: `docs/PRODUCT_V3_PLAN_2026-07-15.md`.
- Current deterministic UAT baseline: 8 checks pass, 11 fail; runtime error check passes after same-origin WebSocket proxy correction.
- Current custom drawings cannot supply selection/editing/undo or required tools.

## In scope

- Verify repository instructions/harness from a clean worktree.
- Record current product-UAT baseline and artifact paths.
- Audit the two candidate drawing projects at exact revisions/versions.
- Build isolated spike surfaces, not production feature integration.
- Evaluate required tools and lifecycle against D-01 through D-11.
- Measure dependency/license/bundle/React-lifecycle/persistence risks.
- Design Sumi-owned drawing state schema and provider adapter contract.
- Recommend one provider, official-primitive fallback, or escalation to chart-library reassessment.
- Produce target frontend component/state architecture for Batch 1.

## Out of scope

- No Indicator Manager implementation.
- No production Replay workspace redesign.
- No migration of existing drawing records.
- No broad backend changes.
- No claim that the selected provider is production-ready before reviewer acceptance.

## Invariants

- Never mutate `backend/sumi.db`; use the deterministic temporary UAT database.
- Do not modify or retag `v2.0.0-rc2`.
- Keep backend indicators authoritative.
- Do not bind Sumi persistence directly to raw provider JSON.
- Do not weaken current failing product assertions.

## Current architecture

- `ReplayPage.tsx` owns active tool and drawing persistence.
- `CandleChart.tsx` owns mouse subscriptions and pending two-point preview.
- `DrawingToolRegistry.ts` renders price lines and ordinary line series.
- `SumiDrawingAdapter.ts` serializes four drawing types.
- `scripts/product-uat.mjs` and `scripts/run-product-uat.sh` provide the deterministic baseline.

## Checkout provenance and affected modules

- User explicitly authorized Batch 0 on the current shared checkout and branch. DEV must not create/switch branches, create a worktree, commit, push, merge, reset, or clean.
- Baseline branch/HEAD: `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2` remains at `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.
- `docs/PROJECT_REVIEW_REPORT_2026-07-15.md` records that the checkout was clean before Reviewer work. The current tracked modifications (`.gitignore`, `docs/AGENTS.md`, `docs/INDEX.md`, `frontend/package.json`, `frontend/src/hooks/useWebSocket.ts`, `frontend/vite.config.ts`) and untracked governance/harness/evidence files are therefore preserved Reviewer inputs, not Batch 0 DEV cleanup targets.
- Preserved untracked inputs include root governance files, V3 documents, this ExecPlan, `scripts/product-uat.mjs`, `scripts/run-product-uat.sh`, `scripts/verify-product.sh`, and `docs/review-artifacts/2026-07-15/*`.
- Batch 0 DEV-owned outputs will be limited to this ExecPlan, isolated spike files under a dedicated non-production surface, spike tests/scripts, and a decision pack/evidence directory. Production `ReplayPage`, chart/drawing implementation, backend contracts, and `backend/sumi.db` are not implementation targets.
- Pre-gate `backend/sumi.db` SHA-256: `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d` (5,152,768 bytes). Recheck after every baseline gate.

## Target design deliverable

Specify interfaces for:

- `DrawingProvider` lifecycle: attach, activate, cancel, select, update, remove, clear, destroy.
- Provider events: created, selected, changed, removed, error.
- Versioned Sumi drawing document independent of provider internals.
- Import/export migration boundary.
- Undo/redo command ownership.
- React mount/unmount and pane ownership.
- Browser UAT selectors and required evidence.

## Milestones

1. **Harness verification — complete:** technical gate passed; product UAT reproduced 8 pass / 11 expected failures with no runtime errors and an unchanged production database hash.
2. **Candidate audit — complete:** exact revisions, installation model, license obligations, maintenance signals, API surface, and bundle observations recorded for both candidates.
3. **Interactive spike — complete:** required tools and lifecycle exercised in Chrome/Playwright at 1440×1000; mandatory failures retained in machine results.
4. **Persistence/lifecycle spike — complete:** deepentropy export/reload/pan/zoom/replay/unmount and difurious export/import/destroy/remount tested.
5. **Decision pack — complete, Reviewer gate passed conditionally:** reviewer independently inspected machine results/screenshots, corrected two over-attributions, rejected both community providers, and authorized only the primitive-provider horizontal-line vertical slice for Batch 1.

## Acceptance mapping

| Acceptance ID | Spike evidence required | Test/UAT evidence |
| --- | --- | --- |
| D-01–D-05 | Required tools and core lifecycle | Both browser results; neither provider passed the full mandatory lifecycle |
| D-06 | Provider event/state support sufficient for Sumi undo/redo | Deepentropy adapter reversal demonstrated; decision pack assigns ownership to Sumi command history |
| D-07 | Pan/zoom/replay/resize behavior | deepentropy browser result passed the combined scenario |
| D-08 | Export/import and versioned mapping | deepentropy semantic export failed; difurious round-trip passed; Sumi v1 JSON Schema added |
| D-09 | Fibonacci levels/labels/direction/edit | Required-tools screenshots and exported level options |
| D-10 | Magnet/constraints | deepentropy stub failed; difurious threshold interaction passed |
| D-11 | No runtime/listener/unmount errors | deepentropy 10 cycles passed; difurious official React app emitted console errors across remounts |

## Verification commands

```bash
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
shasum -a 256 backend/sumi.db
cd spikes/drawing-provider-deepentropy && npm install --force
cd spikes/drawing-provider-deepentropy && npm run build
node scripts/deepentropy-spike-uat.mjs
cd research_repos/lightweight-charts-line-tools-plugin-test-app && npm install
cd research_repos/lightweight-charts-line-tools-plugin-test-app && npm run build
node scripts/difurious-spike-uat.mjs
```

## Rollback and compatibility

All candidate work stays isolated from the production Replay route. Removing the spike must return the repository to the harness-only baseline. No persisted production state is rewritten in Batch 0.

## Risks and mitigations

- Provider README claims may exceed real behavior: verify interactively.
- MPL obligations may affect difurious integration: document affected files/distribution duties before recommendation.
- Provider may expose raw JSON only: require Sumi schema mapping and fixtures.
- Many tools may inflate bundle/performance: measure required-tool subset, not demo bundle alone.
- React listener leaks: repeat mount/unmount and inspect page errors.

## Progress log

- 2026-07-15: reviewer created governance, deterministic product UAT, ADR, acceptance criteria, and this Batch 0 plan.
- 2026-07-15: baseline confirmed at 8 pass / 11 fail, with zero runtime errors after same-origin WebSocket proxy fix.
- 2026-07-15: DEV reread all canonical sources and the browser-control skill. User confirmed shared-checkout one-writer operation; no branch/worktree/git publication actions are permitted.
- 2026-07-15: DEV inventoried current dirty state and attributed it to the preserved Reviewer governance/harness/evidence set using the review report's clean pre-review baseline. Recorded HEAD/tag and the pre-gate production database hash.
- 2026-07-15: `./scripts/verify-v2.sh` passed: backend 75 passed/1 skipped; frontend lint, 9 files/18 tests, and build passed. Initial sandboxed product UAT could not bind localhost; the approved unsandboxed rerun reproduced 8 pass/11 fail with `runtime.no-errors` passing.
- 2026-07-15: deepentropy npm `0.1.1` isolated React spike built and ran. Final browser result: 20 pass/3 fail; body move, specialized persistence, and magnet failed. Ten mount/unmount cycles had no errors.
- 2026-07-15: mandatory deepentropy failures triggered the conditional difurious benchmark. Core plus lines/rectangle/Fibonacci/text exact revisions and official React test app were installed and built.
- 2026-07-15: difurious browser result: 12 pass/8 fail. Full option export/import and magnet passed; Escape left an incomplete tool, move/edit attempt did not change exported points, undo/keyboard delete were absent, and official React remounts emitted console errors.
- 2026-07-15: published decision pack, Sumi provider contract, event mapping, persistence/undo ownership, target architecture, primitive fallback estimate, and versioned JSON Schema. DEV stopped before Batch 1 for Reviewer decision.

## Decision log

- Controlled Replay/Chart UI rebuild selected; Lightweight Charts v5 and backend retained.
- Provider choice is closed for the audited revisions: neither community provider is approved; the official-primitive fallback is conditionally approved for one bounded Batch 1 vertical slice.
- Shared-checkout execution is an explicit user-approved exception to older branch/worktree wording; the updated repository operating model now makes it the default for a single writer.
- Reject deepentropy for production adoption at audited revision/version because D-04, D-08, and D-10 mandatory criteria fail; repository license evidence is incomplete.
- Reject difurious for production adoption at audited revisions because D-02 fails, D-04 was not proven, the official integration does not meet the zero-console-error gate, automated tests are absent, and MPL-2.0 obligations require explicit approval. The demo console errors are not attributed conclusively to core-provider lifecycle cleanup.
- Reviewer approved retaining Lightweight Charts v5 and a Sumi-owned official-primitive provider behind the documented contract. Authorization is limited to a maximum 10-developer-day Batch 1 horizontal-line vertical slice; the full drawing roadmap is not yet approved.
- Deepentropy D-09 is PARTIAL, not PASS: the machine check proves two anchors, not direction editing. Difurious D-11 proves console errors in the official React demo integration, not a core-provider lifecycle leak. Neither correction changes the provider rejection.
- If Batch 1 cannot prove reliable primitive hit testing/coordinate conversion and deterministic lifecycle within its budget, stop and open a base-chart-library reassessment ADR before Batch 2.

## Completion evidence

- Fast gate: PASS (`75 passed, 1 skipped`; frontend `9 files, 18 tests`; lint/build PASS).
- Product UAT baseline: `test-results/product-uat/2026-07-15T14-15-25-051Z/results.json` — 8 pass / 11 known product-gap failures / zero runtime errors.
- Deepentropy: `test-results/drawing-provider-spike/deepentropy/2026-07-15T14-25-40-462Z/` — 20 pass / 3 fail / zero runtime errors, with two screenshots.
- Difurious: `test-results/drawing-provider-spike/difurious/2026-07-15T14-33-53-821Z/` — 12 pass / 8 fail, with two screenshots and captured React console errors.
- Decision pack: `docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md`.
- Schema: `docs/decision-packs/sumi-drawing-document-v1.schema.json`.
- Production database hash remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d` after baseline gates.
- No production Replay route, backend contract, existing persistence record, acceptance criterion, or product-UAT assertion was changed. Reviewer gate is closed; Batch 1 remains responsible for its own ExecPlan and complete acceptance evidence before implementation begins.

## Reviewer verification — 2026-07-15

- Independently inspected both final machine-readable result files and all four final screenshots.
- Confirmed audited local revisions and license metadata for Deepentropy and all five Difurious packages.
- `git diff --check`: PASS.
- `./scripts/verify-v2.sh`: PASS — backend 75 passed/1 skipped; frontend lint, 9 files/18 tests, and production build passed.
- Deepentropy isolated spike `npm run build`: PASS, retaining the documented 578.54 kB minified / 147.08 kB gzip bundle warning.
- Both UAT scripts passed `node --check`; the schema parsed as valid JSON. Full schema fixtures/semantic validation are intentionally deferred to Batch 1 before production persistence.
- `backend/sumi.db` SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.
