# PRO-03 REWORK-06 — Stale preview integrity closure

## Authority and one allowed outcome

Execute only this bounded PRO-03 rework. The goal is to make acceptance of a stale import preview fail closed, without changing the PRO-03 product scope or activating PRO-04.

Read these files completely before editing, in order:

1. `AGENTS.md`
2. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
3. `docs/SESSION_HANDOFF_PROTOCOL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/exec-plans/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md`
6. `docs/reviews/PRO_03_REVIEW_2026-08-10_R3.md`
7. This prompt

Do not rely on chat history. Do not start PRO-04, add dependencies, change acceptance criteria, commit, push, tag, or release. Stop only at a genuine blocker or `IMPLEMENTED — REVIEW PENDING`.

## Rework finding

`ImportWorkflowService.accept_import()` accepts rows classified as `parsed` at preview time without confirming that the corresponding accepted-candle state is unchanged. When a Candle with the same key is created after preview and before accept, the existing-candle branch updates it with stale preview values.

This violates PRO-DATA-03 and PRO-DATA-06. The exact reproduction and evidence are in `docs/reviews/PRO_03_REVIEW_2026-08-10_R3.md`.

## Required implementation

1. Treat a preview as bound to the data state observed during its creation, not merely to its `content_sha256`.
2. Within the accept transaction, revalidate every staged `parsed` candidate against current Candle rows before any mutation. A candidate that was absent at preview but now exists is stale, even if its values happen to match.
3. On any stale candidate, reject the entire accept atomically with an actionable error. Preserve all current Candle rows, do not create `ImportRunMutation` entries, and give the preview/run an honest non-accepted outcome.
4. Remove or make unreachable the current stale overwrite path (`UPDATE` of an existing candle for a previewed parsed row). Do not introduce a conflict-resolution or overwrite feature.
5. Add focused regression coverage that:
   - previews a candidate candle;
   - creates a later/current candle at the same key with different values;
   - attempts acceptance of the original preview;
   - proves rejection, preservation of the later candle, no mutation journal entry, and an honest run status;
   - covers the API error contract if the service behavior is exposed through it.
6. Preserve existing REWORK-03 explicit-confirmation behavior and REWORK-05 function-scoped test isolation.

## Constraints

- `backend/sumi.db` must never be mutated; use temporary or in-memory test databases only.
- Backend services own the rule; FastAPI routes remain transport-only.
- Preserve preview → explicit accept, idempotence, conflict quarantine, adjustment isolation, Weekly derivation, and guarded rollback.
- Do not weaken, remove, rename, duplicate, or make non-blocking any existing UAT assertion.
- Make no unrelated cleanup. If a schema migration or new dependency seems necessary, stop for Reviewer direction.

## Required verification and evidence

Run and record exact results in the PRO-03 ExecPlan and state ledger:

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Push-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_import_classifier.py app/tests/test_import_api.py app/tests/test_cafef_importer.py app/tests/test_import_workflow.py app/tests/test_weekly_aggregator.py -q
& .\.venv\Scripts\python.exe -m pytest app/tests/test_weekly_aggregator.py app/tests/test_import_workflow.py app/tests/test_cafef_importer.py app/tests/test_import_api.py app/tests/test_import_classifier.py -q
Pop-Location
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
.\scripts\verify-product.sh
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

Retain the UAT result, manifest reconciliation, runtime/API outcome evidence, and screenshots at 1440×1000 and 1280×800. Inspect the screenshots. The before/after production database hashes must be identical.

## Handoff

Update `docs/exec-plans/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md` with scope, affected files, acceptance mapping, rollback, progress, decision, test counts, UAT artifact paths, screenshot details, database hashes, deviations, and a Reviewer checklist. Update `docs/AUTONOMOUS_EXECUTION_STATE.md` to `IMPLEMENTED — REVIEW PENDING` only after all DEV gates are genuinely green.

At the Reviewer gate, report only that the workspace is ready for independent review. Do not self-approve.
