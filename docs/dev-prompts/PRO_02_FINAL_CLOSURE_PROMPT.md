# PRO-02 Final Closure Batch

## Mission

Close PRO-02 in one bounded implementation batch after Independent Reviewer rework. Do not optimize for a green counter alone. The output of this session is an evidence-complete handoff named `IMPLEMENTED — REVIEW PENDING`; only an Independent Reviewer may issue `APPROVE`.

Work continuously without routine questions or per-file reports. Stop only for a genuine repository-defined blocker or the Independent Reviewer gate. Do not commit, push, release, add a dependency, migrate data, mutate `backend/sumi.db`, or start PRO-03.

## Mandatory reading order

Read completely before editing:

1. `AGENTS.md`
2. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/reviewer-prompts/PRO_02_REWORK_01.md`
5. `docs/dev-prompts/PRO_02_DAILY_TRADER_WORKFLOW_LOW_MODEL_PROMPT.md`
6. `docs/exec-plans/PRO_02_DAILY_TRADER_WORKFLOW.md`
7. all canonical sources named by `AGENTS.md`

Then inspect the complete current diff, untracked inventory, current tests, latest retained UAT result, and both PRO-02 screenshots. Chat summaries are not authority.

## Operating rule: prove the contract, not the proxy

Before changing code, build a private working matrix for every `PRO-UX-01` through `PRO-UX-09` and every R02-01 through R02-08 requirement:

```text
requirement | implementation location | focused test | browser assertion | retained evidence | remaining gap
```

Do not mark a row satisfied because a nearby proxy passes. URL text alone does not prove store/API/session state. DOM text alone does not prove authoritative index. A green total does not prove assertion quality.

Never make a test green by suppressing, ignoring, broadening, weakening, renaming, deleting, making non-blocking, or replacing an accepted assertion. Expected-error handling must be narrowly scoped to the exact operation and must still retain/classify the outcome.

## Current blocking corrections

Resolve these together, then audit the rest of PRO-02 for the same failure pattern.

### C1 — Restore a clean diff gate

- Remove all trailing whitespace and make `git diff --check` pass.
- Do not use a bulk rewrite that changes unrelated line endings or user work.

### C2 — Remove unconditional error suppression

- Delete the unconditional `console` suppression for messages containing `999999` in `scripts/product-uat.mjs`.
- If the invalid/deleted-session journey produces an expected network/console outcome, classify it only while that exact operation is active, retain it in machine-readable evidence, and prove there are no unexpected runtime/page/console/provider/request failures.
- Prefer fixing the product behavior so an invalid selection is actionable without noisy console errors.

### C3 — Prove one session authority end to end

Use at least two deterministic real replay sessions. For Dashboard Continue and picker selection, prove all of the following agree:

- canonical URL query;
- persisted replay-store session ID, read through an explicit test-only observable or the existing persisted storage contract without adding production debug UI;
- visible Replay product context;
- authoritative session API identity and current index.

Return to the primary session and prove Dashboard → Replay → Journal → Analytics → Replay retains the exact identity and index. Prove Journal and Analytics have no raw numeric Session ID input.

### C4 — Prove browser history completely

Exercise and assert, in order:

1. primary session;
2. picker changes to secondary session;
3. `page.goBack()` restores primary URL/store/DOM/API identity and index;
4. `page.goForward()` restores secondary URL/store/DOM/API identity and index;
5. return to primary and reload;
6. reload restores primary URL/store/DOM/API identity and unchanged index;
7. no session is created or reset during the sequence.

Do not label an assertion back/forward unless both directions are executed and checked.

### C5 — Prove keyboard isolation, including drawings

Before typing replay/drawing shortcut keys into the searchable picker, snapshot:

- authoritative replay current index;
- drawing document bytes or stable normalized drawing state;
- selected session identity.

After typing `ArrowRight`, `Space`, `Delete`, and other relevant owned shortcuts, prove all three snapshots remain unchanged. Close the picker using supported UI behavior and prove normal replay shortcuts still work when focus returns to the intended background.

### C6 — Complete Dashboard state evidence

Retain focused deterministic tests for ready, zero-candle empty, partial, and query-error/retry states. Add missing partial and error/retry coverage. Browser UAT must prove the populated ready journey; controlled component tests may prove empty/partial/error when deterministic browser routing would require test-only product behavior.

### C7 — Replay context contract

Prove the visible Replay header/context contains authoritative symbol, timeframe, adjustment, mode/intent, bar index, `dd/MM/yyyy` date, formatted OHLCV/volume, and explicit `Asia/Ho_Chi_Minh` semantics. The readiness label must be derived from the readiness contract or use honest wording that does not invent PRO-03 freshness/provenance guarantees.

### C8 — Evidence accuracy

- Preserve earlier 293/294 artifacts as rework history.
- Produce one new final artifact; do not cite an incomplete run.
- Record exact commands, exit codes, counts, manifest path/hash, reconciliation arrays/counts, runtime outcomes, screenshot paths/dimensions/hashes, temporary DB identity and cleanup, production DB before/after/current hashes, listeners/process cleanup, and complete changed/untracked inventory.
- Historical progress entries may remain, but label them explicitly as superseded/rework history. The final evidence section must have one unambiguous authoritative run.
- In the state ledger use `IMPLEMENTED — REVIEW PENDING`, never `COMPLETE`, `VERIFIED`, `DONE`, or `APPROVED` for DEV-owned work.

## Required focused coverage

At minimum, focused tests must cover:

- valid URL wins and synchronizes the store;
- valid persisted fallback canonicalizes the URL;
- invalid syntax and deleted IDs clear selection without ID `1` fallback;
- Dashboard Continue from an empty store;
- picker selection synchronizes URL/store/DOM/API;
- back, forward, reload, and route round trip preserve identity/index;
- keyboard isolation preserves index and drawing state;
- Dashboard ready, empty, partial, and error/retry states;
- Vietnamese formatter edge cases and exact Replay labels;
- backtest create rejection and read/list compatibility;
- PRO-01 manifest removal, rename, and blocking-downgrade negative cases.

Review each test assertion for false-green conditions before running the full gates.

## Verification sequence

Run focused tests first. Then run exactly in this order, stopping to diagnose and correct a real failure rather than weakening evidence:

```powershell
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
node --test scripts/product-uat-manifest.test.mjs
powershell -ExecutionPolicy Bypass -File scripts/verify-v2.ps1
powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1
& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-product.sh"
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
git status --short --branch
```

Commands must be bounded. After a long-gate failure, retain the partial result, diagnose it once, apply an evidence-backed correction, and rerun. Do not repeatedly rerun unchanged code.

## Final self-audit before handoff

Inspect the complete diff and answer every item from repository evidence:

- Does every acceptance row prove the actual contract rather than a proxy?
- Were any runtime errors hidden or ignored?
- Were any accepted assertions weakened or changed incompatibly?
- Does `git diff --check` pass?
- Do focused and full gates pass from the current bytes?
- Does the final result reconcile exactly with zero missing, unexpected, duplicate, blocking mismatch, failed, and blocking-failed IDs?
- Are both required screenshots readable at exact dimensions?
- Is `backend/sumi.db` byte-identical?
- Are temporary DBs and owned processes/listeners cleaned up?
- Is all work limited to PRO-02?

If any answer is no, continue correcting inside this session. Do not hand off prematurely.

## Only permitted final status

When every DEV gate is genuinely green, update the ExecPlan and state ledger, then report only:

```text
PRO-02 IMPLEMENTED — REVIEW PENDING
Authoritative artifact: <path>
Checks: <passed>/<declared>; reconciliation clean
DB before/after: <hash> / <hash>
git diff --check: PASS
Full gate: PASS
Next action: Independent Reviewer inspects current workspace and evidence.
```

This is not approval. Stop without committing, pushing, or starting PRO-03.
