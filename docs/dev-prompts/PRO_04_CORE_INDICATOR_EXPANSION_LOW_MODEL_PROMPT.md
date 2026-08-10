# PRO-04 standalone DEV authority — Core Indicator Expansion

You are the implementation session for exactly one bounded batch: PRO-04. Do not rely on chat history.

## Mandatory read order

Read completely before editing:

1. `AGENTS.md`
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
5. `docs/SESSION_HANDOFF_PROTOCOL.md`
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, especially PRO-IND-01..11 and PRO-04
7. `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`
8. `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
9. `docs/reviews/PRO_03_REVIEW_2026-08-10_R2.md`
10. `PLANS.md`

Invocation of this prompt is the user's explicit authorization to start PRO-04 implementation, but not to commit, push, tag, release, add a dependency, mutate production data, or begin PRO-05.

## Startup gate

Before product edits:

- confirm PRO-03 has an Independent Reviewer `APPROVE` and PRO-04/PRO-05 implementation is absent;
- confirm branch/commit, preserve every existing dirty/untracked file, and inventory overlaps;
- verify the transferred DB and reviewer artifact hashes against the machine-transfer handoff;
- confirm Python 3.12+, Node/npm, backend environment, and frontend dependencies;
- run `git diff --check` and the focused existing indicator tests;
- update `docs/AUTONOMOUS_EXECUTION_STATE.md` to name PRO-04 as active, this ExecPlan/prompt, the startup evidence, and the exact next milestone.

If the transferred workspace is missing PRO-03 files, the DB hash unexpectedly differs, or existing changes overlap PRO-04 files in an unexplained way, stop with a durable blocker. Do not reset, clean, restore, delete, or reconstruct missing work.

## Execute continuously

Implement every milestone and acceptance mapping in `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`. Begin with contract inventory and failing focused tests. Keep backend calculations authoritative. Replace the unsafe frontend EMA fallback with an exhaustive released-definition catalog. Complete SMA, Bollinger Bands, ATR, and backend Volume SMA; keep raw Volume distinct. Cover semantic output mapping, pane/series behavior, warm-up/gaps, no-future boundary, multiple instances, settings, visibility, ordering, styles, persistence, reload, resume, navigation, and cleanup.

Make reasonable in-scope decisions from repository evidence. Do not ask the user routine questions or paste per-command logs. Do not weaken acceptance criteria or accepted UAT assertions. Do not add PRO-05 definitions to the released catalog.

After focused tests are green, run the full technical gate and deterministic Product UAT against temporary databases. Inspect the retained 1440×1000 and 1280×800 screenshots, reconcile the manifest, classify runtime/request errors, confirm listeners and temporary databases are cleaned up, and prove `backend/sumi.db` has the same SHA-256 before and after.

Update the ExecPlan and state ledger after each milestone with exact commands, counts, decisions, deviations, artifacts, hashes, and cleanup evidence. Repository files are the handoff; the user must not relay technical context manually.

## Stop boundary

Stop only for a protocol-defined genuine blocker or when all DEV work and evidence are complete. At success, record only:

`IMPLEMENTED — REVIEW PENDING`

Do not self-approve. Do not commit or push. Do not start PRO-05. Tell the user only that PRO-04 has reached the Independent Reviewer gate and that the reviewer should inspect the current workspace.
