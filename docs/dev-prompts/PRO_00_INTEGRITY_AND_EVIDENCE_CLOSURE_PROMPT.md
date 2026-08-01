# PRO-00 Integrity and Evidence Closure — Standalone DEV Prompt

You are the dedicated DEV task for Sumi **PRO-00 only**. You have no access to the Reviewer conversation. This file is your complete implementation authority.

## 1. Outcome

Deliver one bounded vertical capability:

1. Scanner-created replay has an explicit blind-practice or signal-review intent.
2. Blind practice never exposes future signal information through API payloads, chart labels, markers, badges, panels, websocket state, or other derived UI.
3. Signal context is revealed at the authorized candle only.
4. Product UAT uses a checked-in fail-closed assertion manifest instead of treating an ignored historical result file as optional.
5. The current dirty worktree and production database are preserved and fully accounted for.

Do not start PRO-01. Do not declare Sumi Professional, product-complete, release-ready, or "TradingView-like".

## 2. Repository and task rules

Work only in:

```text
E:\Workspace\sumi
```

Use the current checkout and branch. Do not:

- create or switch a branch;
- create a worktree;
- reset, restore, clean, delete, or overwrite unrelated work;
- stage, commit, push, merge, tag, package, or publish;
- modify V2 historical release/tag evidence to imply Post-V3 completion;
- weaken, remove, rename, or make an accepted assertion non-blocking;
- change acceptance criteria to make implementation easier;
- mutate `backend/sumi.db`;
- add telemetry or transmit user trading, strategy, order, or journal data;
- add a dependency without stopping for Reviewer approval.

Only this DEV task may write the checkout while PRO-00 is active. The Reviewer task will not concurrently edit overlapping files.

## 3. Mandatory reading

Read completely before planning or coding:

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/INDEX.md`
4. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
5. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
6. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
7. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
8. `docs/DEVELOPMENT_OPERATING_MODEL.md`
9. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
10. `docs/V3_ACCEPTANCE_MATRIX.md`
11. `docs/V3_EVIDENCE_INDEX.md`
12. `docs/V3_RELEASE_CANDIDATE_NOTES.md`
13. `docs/tester/REAL_TRADER_PRODUCT_REVIEW_AND_GAP_ANALYSIS.md`
14. `docs/tester/OTHER_FEATURES_REVIEW_AND_GAP_ANALYSIS.md`
15. `docs/tester/05_SUMI_V3_FINAL_COMPREHENSIVE_REVIEW.md`

Treat `docs/tester/` as research input only. Reproduce claims before using them as evidence.

## 4. Initial state that must be preserved

At prompt creation time, the known dirty state is:

```text
 M scripts/product-uat.mjs
?? docs/tester/
?? scripts/run-product-uat.ps1
```

The known modification in `scripts/product-uat.mjs` changes a required baseline read to:

```javascript
await readJson(...).catch(() => null)
```

Do not reset or silently discard this change. It is in PRO-00 scope only because the target design replaces the ignored-artifact dependency with a checked-in fail-closed manifest. Record the original diff and the final disposition in the ExecPlan.

Before implementation, record:

```powershell
git status --short --branch
git rev-parse HEAD
git rev-parse origin/master
git rev-list -n 1 v2.0.0-rc2
git diff -- scripts/product-uat.mjs
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

Also inventory untracked files without editing, deleting, moving, staging, or assuming ownership of them.

## 5. Required ExecPlan

Before product or test code, create:

```text
docs/exec-plans/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE.md
```

Follow `PLANS.md` exactly. Include:

- outcome and observed defect;
- in/out of scope;
- V3 R-01 and PRO-G/PRO-INT acceptance mapping;
- current API, persistence, frontend, websocket, UAT, and test paths;
- target contracts below;
- milestones with independently testable exit criteria;
- exact verification commands;
- compatibility and rollback;
- dirty-state preservation;
- risks and stop conditions;
- progress, decision, deviations, and completion logs.

Do not write an aspirational plan that delegates contract decisions back to implementation. Use the decisions in this prompt.

## 6. Scope

### In scope

- Scanner signal → Replay session creation.
- Replay session response serialization for scanner source context.
- Blind/review mode product controls and visible labeling.
- Create/get/resume/advance/rewind and websocket source-context behavior.
- Frontend removal of raw `source_payload` as scanner display authority.
- Backward behavior for existing scanner sessions.
- Checked-in UAT assertion manifest and fail-closed validation.
- Product UAT result metadata and focused assertions.
- Tests, documentation, ExecPlan, evidence, and handoff.

### Out of scope

- PRO-01 backtest metric changes.
- Dashboard, session picker, localization, data catalog, indicators, risk tools, journal expansion, or strategy UX.
- Broad replay, scanner, persistence, database, or websocket rewrites.
- New dependencies.
- Data migration that can lose or rewrite existing session history.
- Commit, tag, push, release, or publication.

## 7. Scanner replay contract

### 7.1 Request

Extend Scanner replay creation with:

```text
replay_intent: "blind_practice" | "signal_review"
```

Rules:

- omitted value defaults to `blind_practice`;
- unknown values fail typed request validation;
- frontend presents two understandable actions or an explicit mode selection;
- blind practice is the default/recommended action;
- request and persisted audit state retain the selected intent.

Do not reuse an unrelated replay/backtest `mode` field with different semantics.

### 7.2 Sanitized response

Add a versioned server-authoritative `source_context` view:

```text
source_type: string | null
replay_intent: "blind_practice" | "signal_review" | null
reveal_at_index: integer | null
revealed: boolean
signal: null | {
  timestamp: canonical API timestamp
  type: string
  strategy: string
  price: number
  regime: string | null
}
```

Use typed backend and frontend schemas. Validate stored JSON before translating it into this view.

### 7.3 Blind-practice behavior

- Determine `reveal_at_index` from the actual session candle sequence and selected signal timestamp, not from a frontend estimate.
- For every response where `current_index < reveal_at_index`, return `revealed: false` and `signal: null`.
- The raw scanner `source_payload` must not carry future signal fields to the frontend before reveal.
- The chart, badges, side panels, markers, accessible names, hidden DOM, query cache, and persisted frontend state must contain no premature signal values.
- At `current_index == reveal_at_index`, return `revealed: true` and the sanitized signal exactly once.
- Advancing beyond the signal keeps it revealed.
- Rewinding before it returns the server response to the unrevealed view; frontend renders only that view.
- Reload and resume derive visibility from persisted current index and server state.
- Repeated navigation produces no duplicate signal marker, stale badge, or race.

### 7.4 Signal-review behavior

- Start at the signal candle, not in the hidden lookback period.
- Return `revealed: true` with sanitized signal context.
- Label the workspace clearly as signal review.
- Do not describe this session as blind practice.
- Existing no-future rules still apply to candles and indicators after the signal candle.

### 7.5 Persistence and compatibility

- Full scanner source data may remain in local persistence for audit.
- Frontend must stop parsing raw `source_payload` as its signal display source.
- Non-scanner replay sessions preserve their existing behavior.
- Existing scanner sessions without `replay_intent` are interpreted as `blind_practice`.
- Prefer a compatibility view over a destructive database migration.
- If a schema migration is unavoidable or existing session data cannot be represented safely, stop at the escalation gate before implementing it.

### 7.6 Response-path audit

Inspect every path that returns or broadcasts replay session state. The solution is incomplete if one of create, get, resume, advance, rewind, websocket, or another session endpoint still exposes raw future source data.

Do not accept frontend-only hiding as closure.

## 8. UAT evidence contract

### 8.1 Checked-in manifest

Create a repository-controlled manifest at:

```text
scripts/fixtures/product-uat-v3-baseline.json
```

Minimum schema:

```json
{
  "schema_version": 1,
  "baseline_name": "sumi-v3-accepted-regression-baseline",
  "acceptance_contract_revision": "2026-07-31",
  "assertions": [
    {
      "id": "stable-assertion-id",
      "blocking": true,
      "acceptance_ids": ["G-01"]
    }
  ]
}
```

Requirements:

- audit the current harness and available retained results before populating it;
- every accepted baseline ID appears exactly once;
- every entry has a boolean `blocking` and at least one acceptance ID;
- invalid schema, missing file, empty assertions, duplicate ID, absent accepted ID, removed entry, or blocking downgrade fails before a successful result can be declared;
- additive PRO-00 assertions map to V3 R-01 and applicable PRO-INT IDs;
- do not copy unsupported historical counts merely because a report claims them;
- record the source and reconciliation decision in the ExecPlan.

If available evidence cannot establish a trustworthy baseline, stop and return an evidence blocker rather than inventing one.

### 8.2 Harness behavior

- Remove reliance on an ignored `test-results/.../results.json` file as the only baseline authority.
- Remove fail-open `.catch(() => null)` behavior.
- Validate the checked-in manifest strictly before executing or evaluating UAT.
- Compare actual results against manifest IDs and blocking semantics.
- Keep new assertions additive.
- Do not rename an existing assertion to make reconciliation easier.
- A failed run must still retain its machine-readable results and diagnostic evidence.

### 8.3 Result metadata

Each product UAT result must retain:

- manifest path/version/hash;
- baseline and actual assertion counts;
- missing, unexpected, duplicate, and blocking-mismatch IDs;
- assertion-to-acceptance mapping;
- runtime, console, page, provider, request, and failed API outcomes;
- temporary database identity;
- production DB before/after hashes;
- requested sustained duration and actual samples when applicable;
- screenshot metadata and viewport.

## 9. Required tests

### 9.1 Backend

Add focused tests for:

- request default and enum validation;
- blind session response before signal;
- exact reveal boundary;
- after signal;
- rewind before signal;
- reload/resume;
- signal review initial index and context;
- legacy missing-intent scanner session;
- malformed or incomplete stored source payload;
- non-scanner compatibility;
- every session-returning or websocket path;
- no raw future fields in serialized payload.

Assertions must inspect complete serialized responses, not only candle arrays.

### 9.2 Frontend

Add focused tests for:

- Scanner default blind action and explicit review action;
- request payload intent;
- no parsing/display of raw `source_payload`;
- unrevealed/revealed source context;
- review label;
- rewind removing pre-boundary context;
- reload/resume;
- no duplicate marker or badge;
- keyboard/accessibility names containing no hidden future values.

### 9.3 UAT harness

Test or deterministically exercise:

- missing manifest;
- malformed manifest;
- empty assertion set;
- duplicate IDs;
- missing accepted ID;
- renamed/removed ID;
- blocking downgrade;
- additive valid assertion;
- failed UAT retaining results.

Do not create a test that passes only because it reads the same malformed source through the same unchecked code path.

### 9.4 Browser UAT

Use deterministic temporary data and real UI actions:

1. Run Scanner and select a historical signal.
2. Start blind practice.
3. Inspect the initial network/session payload and visible DOM.
4. Prove timestamp, price, strategy, regime, entry label, and marker are absent.
5. Advance to one candle before signal and prove absence.
6. Advance to the signal and prove one reveal.
7. Advance, rewind before signal, reload, and resume.
8. Start signal review and prove it begins at the signal with an honest review label.
9. Capture required 1440×1000 and 1280×800 screenshots.
10. Retain zero-error console/page/provider/request evidence.

Hidden test-only state declarations are not product evidence.

## 10. Milestones

### Milestone 1 — Baseline and design

- Mandatory reading complete.
- Initial provenance, dirty state, and DB hash recorded.
- Complete response-path and UAT baseline inventory recorded.
- ExecPlan target design and acceptance mapping complete.

Exit: no unresolved discoverable contract question remains.

### Milestone 2 — Backend integrity

- Request enum/default implemented.
- Server-authoritative sanitized source context implemented.
- All response paths and compatibility cases covered.

Exit: focused backend suite proves complete-payload no-future behavior.

### Milestone 3 — Frontend behavior

- Scanner exposes honest blind/review actions.
- Replay consumes only sanitized context.
- Visible boundary, rewind, reload, resume, and duplicate behavior pass.

Exit: focused frontend tests pass.

### Milestone 4 — Evidence authority

- Checked-in manifest exists and is strictly validated.
- Missing/malformed/removal/downgrade cases fail closed.
- Results retain required reconciliation and hash metadata.

Exit: harness-focused tests and deliberate failure run retain evidence.

### Milestone 5 — Product gate and handoff

- Browser scenarios pass at both required viewports.
- All technical/product gates pass.
- DB hash is unchanged.
- Diff and evidence self-review is complete.

Exit: DEV stops at Reviewer gate.

## 11. Verification

Run focused commands discovered from the repository and record them verbatim in the ExecPlan. Then run at minimum:

```powershell
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

```bash
cd backend && pytest
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
```

If shell compatibility requires the existing Windows runner, inspect it first, record that it was pre-existing and untracked, and do not silently promote or rewrite it outside PRO-00 needs.

After all gates, run the SHA-256 check again and compare the exact value.

Review screenshots visually. A passing assertion count without visible product review is insufficient.

## 12. Required acceptance mapping

At minimum map implementation and evidence to:

- V3 G-01 through G-05;
- V3 R-01;
- PRO-G-01 through PRO-G-10;
- PRO-INT-01 through PRO-INT-10.

Other accepted V3 R/I/D/T assertions remain blocking regressions.

## 13. Stop and escalation conditions

Stop without broadening scope if:

- a safe solution requires destructive migration or can lose existing sessions;
- current accepted UAT IDs cannot be reconstructed from trustworthy repository/evidence sources;
- a backend contract outside Scanner/Replay must change materially;
- websocket behavior requires an unrelated architecture rewrite;
- a new dependency appears necessary;
- acceptance criteria conflict or cannot be met;
- production DB changes;
- a pre-existing uncommitted file would need to be deleted or overwritten outside the documented in-scope UAT change;
- fixing a discovered P0/P1 requires PRO-01 or later work.

Record the exact blocker, evidence, attempted safe alternatives, and required Reviewer decision.

## 14. Self-review and handoff

Before returning:

- inspect every changed and untracked file;
- compare the diff to the ExecPlan and acceptance IDs;
- verify no unrelated cleanup or later-batch work;
- verify no test, assertion, error capture, duration, or blocking semantic was weakened;
- verify frontend has no raw scanner payload display path;
- verify every replay response path is sanitized;
- verify failure evidence is retained;
- update ExecPlan progress, decisions, deviations, risks, verification, and completion evidence.

Handoff must report:

- outcome and acceptance IDs;
- exact files changed;
- public contract and compatibility behavior;
- exact test commands and counts;
- UAT artifact path, manifest hash, and screenshots;
- DB before/after hashes;
- known failures and accepted limitations;
- recommended Reviewer checks;
- explicit statement that no acceptance test was weakened.

Final line:

```text
PRO-00 DEV COMPLETE — STOPPED AT REVIEWER GATE
```

Do not continue into PRO-01.
