# PRO-03 DEV authority — Data Catalog and Import Quality

You are the sole implementation writer for PRO-03 in the current checkout. Do not rely on chat history. Work continuously until a genuine stop condition or `IMPLEMENTED — REVIEW PENDING`; do not ask the user to relay ordinary progress or test output.

## Mandatory reading before edits

Read completely, in this order:

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
5. `docs/SESSION_HANDOFF_PROTOCOL.md`
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
7. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
8. `PLANS.md`
9. `docs/program/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md`
10. `docs/exec-plans/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md`

Then inspect the current Git status/diff and every current import/catalog caller named by the ExecPlan. Preserve all pre-existing changes. Do not reset, clean, restore, delete, or overwrite them. Record the starting inventory and production database SHA-256 in the ExecPlan/state ledger before product edits.

## Authorized outcome and scope

Implement exactly PRO-03 and `PRO-DATA-01` through `PRO-DATA-07` as specified in the active ExecPlan. The required vertical capability is catalog → preview → blocked/explicit accept → idempotent outcome → history → guarded rollback, plus deterministic Daily→Weekly provenance.

Follow the target design, classification contract, `VN_TRADING_WEEK_V1`, milestones, acceptance mapping, and rollback rules in the ExecPlan. If current code contradicts a detail, preserve the acceptance outcome and invariants, document the evidence and decision in the ExecPlan before deviating, and keep the deviation within PRO-03.

## Non-negotiable implementation rules

- Never use or mutate `backend/sumi.db` in tests, migrations, scripts, or UAT. Use an explicit temporary database and verify the production hash before/after.
- No direct importer, API, seed, or batch script may remain as a silent last-wins mutation bypass.
- Preview must not mutate accepted Symbol/Candle data. Acceptance must bind to the immutable preview/run checksum and be atomic.
- Ambiguity, malformed required values, and unresolved conflicts fail closed; no quiet partial acceptance and no keep-last behavior.
- Adjusted and unadjusted histories remain isolated.
- Repeated semantic input is a recorded no-op, not a rewrite.
- Weekly values derive only from accepted Daily members under the versioned rule and retain ordered provenance.
- Rollback restores exact before-images only when safe; otherwise reject with a specific reason.
- Business rules belong in services/domain modules, not FastAPI routes or React components.
- Use Alembic for persisted schema. Legacy candles remain readable and are labeled with honest unknown/legacy provenance.
- Add no dependency without stopping for Reviewer/user authorization and license/security evidence.
- Preserve all PRO-00–PRO-02 behavior and the fail-closed UAT manifest. Never weaken an assertion or suppress errors globally.
- Do not implement network provider/sync, intraday, indicators, PRO-04, or later work.
- Do not commit, push, tag, release, package, or publish.

## Required execution loop

1. Update the ExecPlan current-architecture inventory and starting hashes/status if inspection finds differences.
2. Implement one milestone at a time with focused tests before moving on.
3. After each milestone, update the ExecPlan progress/decision/deviation log and `docs/AUTONOMOUS_EXECUTION_STATE.md` with the exact durable control point.
4. Add typed backend schemas and frontend types; make user-visible copy Vietnamese and actionable.
5. Extend `scripts/product-uat.mjs` and the checked-in manifest additively with visible user actions and response-payload assertions for every PRO-DATA ID. Include negative cases proving accepted data did not change.
6. Run the exact focused and full commands from the ExecPlan. Diagnose a failure once, apply one evidence-backed correction, and rerun bounded checks; do not loop blindly.
7. Inspect retained 1440×1000 and 1280×800 screenshots, machine-readable results, runtime/API/console errors, temporary DB cleanup, owned listener cleanup, manifest reconciliation, and final diff.
8. Record exact counts, artifact paths, screenshot dimensions/SHA-256, database before/after SHA-256, cleanup evidence, changed-file inventory, and known limitations in the ExecPlan.
9. Self-review the complete diff against every `PRO-DATA-01`–`07` row and verify no later-batch or unrelated changes are included.

## Genuine stop conditions

Stop only for a condition listed in `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`, including destructive migration/data-loss risk, dependency/provider/license/security choice, acceptance conflict, unavoidable overlap with unknown user changes, production DB mutation, or the same evidence-backed blocker after one correction and a second bounded verification. Write the blocker, evidence, and one exact next action to the ledger before stopping.

Do not stop merely because a test fails, implementation is large, context is long, or a routine design choice is required by the ExecPlan.

## Final handoff contract

When all DEV gates are green, update the ledger to `reviewer-gate`, finish the ExecPlan completion evidence and Reviewer checklist, and report only:

`IMPLEMENTED — REVIEW PENDING`

Include concise evidence paths and the instruction: `Return to the Independent Reviewer session and say: Antigravity đã đến Reviewer gate. Review PRO-03 từ workspace hiện tại.` Do not approve yourself and do not start PRO-04.
