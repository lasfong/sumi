# Sumi documentation index

## Canonical V3 documents

Read these in order for all new development:

1. `docs/PRODUCT_V3_PLAN_2026-07-15.md` — product outcome, controlled UI rebuild, batches, risks.
2. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` — measurable release contract.
3. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md` — B/C decision and retained boundaries.
4. `docs/DEVELOPMENT_OPERATING_MODEL.md` — reviewer/DEV roles, worktree and batch workflow.
5. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md` — evidence-backed current-state baseline.
6. `AGENTS.md` — durable repository instructions for Codex and developers.
7. `PLANS.md` — required ExecPlan structure.

Active batch plans belong in `docs/exec-plans/`. Reviewer gate reports belong in `docs/reviews/`. Prompts handed to the separate DEV task belong in `docs/dev-prompts/`; use the file as the handoff authority instead of pasting a long prompt into task conversation.

Reviewer/orchestrator task handoffs belong in `docs/reviewer-prompts/` and must be standalone for a new task with no conversation history.

## Canonical Post-V3 professionalization program

- `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` — canonical product target, verified gap register, Professional acceptance contract, ordered PRO-00 through PRO-12 program, release gates, and deferred backlog.
- `docs/PROFESSIONALIZATION_HANDOFF_2026-08-01.md` — current cross-machine checkout, status, reading order, continuation command, and Git safety handoff.
- `docs/dev-prompts/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE_PROMPT.md` — completed standalone PRO-00 implementation authority and preserved contract.
- `docs/exec-plans/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE.md` — living implementation/evidence record; implementation is committed, final verification and Reviewer decision remain pending.
- `docs/dev-prompts/PRO_00_VERIFICATION_CONTINUATION_PROMPT.md` — the only currently authorized continuation prompt; verification and Reviewer handoff only.

The historical V3 Batch 0–5 closure remains recorded below. A later exploratory browser review found that a Scanner-created replay can expose future signal timestamp, price, strategy, regime, and entry metadata before the replay reaches the signal candle. This reopens V3 R-01 for current release eligibility even though future candle slicing may remain correct.

Until PRO-00 is independently approved:

- V3 release acceptance is conditional;
- no later PRO batch is authorized;
- Sumi must not be tagged, published, or described as a Professional-complete product;
- reports under `docs/tester/` remain research inputs rather than canonical release evidence.

Current gate:

- No development batch remains open under the historical V3 roadmap. Batch 5 and the V3 acceptance contract were independently approved and closed on 2026-07-22. PRO-00 implementation is present in commit `55ec5f9`; its final canonical verification and independent Reviewer decision remain pending. No later PRO batch is authorized. Any later commit, tag, push, or release publication is a separate explicitly authorized operation.

- `docs/reviews/BATCH_1_REVIEW_2026-07-16.md` — Batch 1 approved and closed after final independent verification.
- `docs/dev-prompts/BATCH_1_REVIEW_HARDENING_PROMPT.md` — completed first hardening prompt.
- `docs/dev-prompts/BATCH_1_FINAL_CLOSURE_PROMPT.md` — completed final closure prompt.
- `docs/dev-prompts/BATCH_2_INDICATOR_MANAGER_PROMPT.md` — completed initial Batch 2 implementation prompt.
- `docs/reviews/BATCH_2_REVIEW_2026-07-16.md` — Batch 2 approved and closed after bounded hardening and independent verification.
- `docs/dev-prompts/BATCH_2_REVIEW_HARDENING_PROMPT.md` — completed Batch 2 hardening prompt.
- `docs/dev-prompts/BATCH_3_PROFESSIONAL_DRAWING_MVP_PROMPT.md` — completed initial Batch 3 implementation prompt.
- `docs/dev-prompts/BATCH_3_REVIEW_HARDENING_PROMPT.md` — completed first Batch 3 hardening prompt.
- `docs/dev-prompts/BATCH_3_SECOND_REVIEW_CLOSURE_PROMPT.md` — completed B3-R07–B3-R10 closure prompt.
- `docs/reviews/BATCH_3_REVIEW_2026-07-18.md` — Batch 3 Drawing MVP approved and closed after independent second-closure verification.
- `docs/reviewer-prompts/REVIEWER_ORCHESTRATOR_HANDOFF_BATCH_3_2026-07-18.md` — completed Batch 3 Reviewer/Orchestrator handoff.
- `docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md` — completed initial Batch 4 implementation prompt.
- `docs/reviews/BATCH_4_REVIEW_2026-07-18.md` — Batch 4 approved and closed after B4-R01–B4-R03 and independent 254-ID verification.
- `docs/dev-prompts/BATCH_4_REVIEW_CLOSURE_PROMPT.md` — completed Batch 4 closure prompt.
- `docs/dev-prompts/BATCH_5_PRODUCT_HARDENING_V3_RC_PROMPT.md` — completed initial Batch 5 DEV prompt.
- `docs/reviews/BATCH_5_REVIEW_2026-07-19.md` — Batch 5 and V3 acceptance approved and closed after B5-R01–B5-R07 and independent sealed-evidence verification.
- `docs/dev-prompts/BATCH_5_REVIEW_CLOSURE_PROMPT.md` — completed first closure authority; DEV stopped correctly at its R04 condition.
- `docs/dev-prompts/BATCH_5_R04_CONTINUATION_PROMPT.md` — completed B5-R01–B5-R05 continuation; product behavior accepted at Reviewer gate.
- `docs/dev-prompts/BATCH_5_EVIDENCE_SEALING_PROMPT.md` — completed evidence-only B5-R06–B5-R07 sealing authority.
- `docs/V3_ACCEPTANCE_MATRIX.md` — independently approved evidence mapping for every V3 acceptance ID.
- `docs/V3_RELEASE_CANDIDATE_NOTES.md` — bounded changes, measured outcome and Reviewer closure record.
- `docs/V3_VERIFICATION_AND_RECOVERY.md` — reproducible local verification, backup and restored-copy procedure.
- `docs/V3_EVIDENCE_INDEX.md` — canonical Batch 5 manifest/results/recovery/screenshot locations.

## V2 baseline documents

The V2 specifications, completion plans, checklists, status reports, and release evidence document how `v2.0.0-rc2` was built and technically verified. They are historical baseline evidence, not the acceptance contract for V3 product quality.

Important baseline references:

- `docs/SPEC_V2.md`
- `docs/MANUAL_REPLAY_SPEC.md`
- `docs/BACKTEST_ENGINE_SPEC.md`
- `docs/ACCEPTANCE_CRITERIA_V2.md`
- `docs/PRODUCT_COMPLETION_PLAN_2026-07-04.md`
- `docs/RELEASE_EVIDENCE_2026-07-04.md`
- `docs/RELEASE_VERIFY_LOG_2026-07-04.md`
- `docs/HANDOFF_REPORT_2026-07-03.md`

If V2 completion language conflicts with the 2026-07-15 review or V3 acceptance criteria, the V3 documents govern new work.

## Archived material

Pre-V2 documents live under `docs/archive/pre_v2/` and are historical only.

## Operational notes

- Raw market files belong under `data/raw/`.
- Automated tests/UAT must not mutate `backend/sumi.db`.
- Product UAT artifacts belong under ignored `test-results/`; selected review evidence may be copied to `docs/review-artifacts/`.
- Local research clones belong under ignored `research_repos/`.
