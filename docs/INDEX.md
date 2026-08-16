# Sumi documentation index

## Read order for every new session

1. `AGENTS.md` — repository rules and non-negotiable invariants.
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md` — destination bootstrap, transfer integrity, program summary, and exact next prompt.
3. `docs/AUTONOMOUS_EXECUTION_STATE.md` — volatile authority: approvals, active batch, control point, and exact next action.
4. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md` — two-session DEV/REVIEW state machine, file ownership, and detailed DoD.
5. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md` — core DEV/Reviewer workflow and stop rules.
6. `docs/SESSION_HANDOFF_PROTOCOL.md` — workspace-based handoff contract.
7. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` — stable PRO-00 through PRO-12 roadmap and acceptance contract.
8. The active or prepared-next dossier under `docs/program/`.
9. The ExecPlan and DEV/Reviewer prompt named by the ledger.

The workspace, not chat history, is the handoff channel. Only the batch named **Active batch** by the ledger is authorized.

## Canonical product and architecture sources

- `docs/PRODUCT_V3_PLAN_2026-07-15.md` — V3 product outcome and controlled UI rebuild.
- `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md` — measurable V3 release contract.
- `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md` — replay UI decision and retained boundaries.
- `docs/DEVELOPMENT_OPERATING_MODEL.md` — reviewer/DEV roles and bounded-batch workflow.
- `docs/PROJECT_REVIEW_REPORT_2026-07-15.md` — evidence-backed baseline.
- `PLANS.md` — required ExecPlan structure.
- `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` — canonical Professionalization outcome, acceptance IDs, ordered program, risks, and release policy.

## Execution records

- `docs/program/` — stable batch dossiers. Future dossiers frame outcomes but do not authorize implementation.
- `docs/exec-plans/` — detailed, living plans and evidence for one bounded batch.
- `docs/dev-prompts/` — standalone implementation authority for a DEV session with no chat context.
- `docs/reviewer-prompts/` — bounded Reviewer/rework authorities.
- `docs/reviews/` — dated independent `APPROVE` or `REWORK` records.
- `docs/review-artifacts/` — selected durable review evidence; full local UAT artifacts remain under ignored `test-results/`.

PRO-00–PRO-02 and historical V3 Batch 0–5 records are completed evidence, not current instructions. `docs/PROFESSIONALIZATION_HANDOFF_2026-08-01.md` is explicitly superseded and retained only for provenance.

Current boundary: PRO-09 is independently approved and closed by `docs/reviews/PRO_09_REVIEW_2026-08-16.md`. PRO-10 is independently approved and closed by `docs/reviews/PRO_10_REVIEW_2026-08-16.md`; PRO-11 remains unauthorized. Start a new DEV session with `docs/dev-prompts/ANTIGRAVITY_DEV_SESSION_INIT_PROMPT.md` and a REVIEW session with `docs/reviewer-prompts/ANTIGRAVITY_REVIEW_SESSION_INIT_PROMPT.md`. `docs/AUTONOMOUS_EXECUTION_STATE.md` is the volatile execution authority; `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md` remains the transfer entrypoint.

## Research and historical material

- `docs/tester/` contains non-authoritative exploratory research and test ideas. Findings become requirements only after adoption into the active acceptance mapping/ExecPlan.
- V2 specifications, completion plans, checklists, and release evidence are historical baselines. They do not override V3/PRO acceptance.
- `docs/archive/pre_v2/` is historical only.
- `docs/BacktestSample/`, `docs/FEATURE_MATRIX_RESEARCH.md`, and similar research documents are inputs, not batch approval.

## Operational rules

- Raw market files belong under `data/raw/`.
- Automated tests and UAT must never mutate `backend/sumi.db`.
- Local research clones belong under ignored `research_repos/`.
- Do not infer authorization to commit, push, tag, package, publish, or start a later PRO from any plan or approval record.
