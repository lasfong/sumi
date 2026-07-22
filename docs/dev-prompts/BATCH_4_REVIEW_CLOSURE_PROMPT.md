# Batch 4 review closure — standalone DEV prompt

You are the DEV task for one bounded Batch 4 closure in the Sumi repository. Work only on B4-R01–B4-R03 from `docs/reviews/BATCH_4_REVIEW_2026-07-18.md`. Batch 4 is open; Batch 5 is not authorized.

## Mandatory reading before any product change

Read completely, in this order:

1. `AGENTS.md`
2. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
5. `docs/DEVELOPMENT_OPERATING_MODEL.md`
6. `PLANS.md`
7. `docs/dev-prompts/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW_PROMPT.md`
8. `docs/exec-plans/BATCH_4_INTEGRATED_TRADING_PRACTICE_WORKFLOW.md`
9. `docs/reviews/BATCH_4_REVIEW_2026-07-18.md`

Inspect the current dirty tree and preserve all unrelated user/reviewer changes. Do not create a branch/worktree, stage, commit, push, merge, reset, clean, checkout or tag. Do not move or rewrite `v2.0.0-rc2`. Do not mutate `backend/sumi.db`; use in-memory or temporary databases.

Before coding, append the closure scope, affected modules, acceptance mapping, exact tests and rollback strategy to the existing Batch 4 ExecPlan. Record a red focused reproduction for each defect before implementing the fix.

## B4-R01 — chronological semantics for multi-step advance

Correct `ReplayService.next_candle()` so advancing N candles is lifecycle-equivalent to advancing one candle N times in chronological order. A LIMIT touched only on a crossed intermediate candle must execute on the earliest eligible candle with the same price, execution date, order status, cash, position, realized/unrealized P&L, T+2 availability, marker and bankruptcy/MTM result as repeated single steps.

Retained executions remain ledger facts. After rewind, forward traversal or a later multi-step jump must project the same execution exactly once and must not create a duplicate. Handle destination/end-of-session boundaries explicitly. Keep business logic in services; do not patch around this in React or UAT.

Add focused backend cases for at least:

- intermediate-only LIMIT hit during a multi-step jump;
- equality of one multi-step advance and repeated single steps using separate identical sessions;
- no hit, destination hit, earliest of multiple eligible crossed candles and completion-boundary behavior;
- rewind then multi-step forward with unchanged execution identity/count;
- relevant MTM/cash/position/T+2 state equality.

## B4-R02 — authoritative checklist date binding

Make the backend validate checklist context date against the authoritative candle at the session's current index, using the exact session symbol/timeframe/adjustment/date-range candle series. Normalize only to the product's declared canonical date representation; do not accept a date simply because it is a nonempty string.

Reject wrong valid dates, impossible/malformed dates, stale indices and cross-session/symbol contexts without inserting or mutating a journal entry. Preserve readable legacy opaque notes and the existing schemaVersion 1 envelope; do not add a migration or rewrite stored rows.

Add service/API tests for the accepted exact date and every rejected category above. If authoritative candle resolution is ambiguous or missing for a retained session, stop and return to Reviewer rather than guessing.

## B4-R03 — exact additive evidence

Keep all 246 baseline assertion IDs from `test-results/product-uat/2026-07-18T07-22-47-858Z/results.json` exactly once with unchanged names, pass values and blocking status.

Add blocking UAT through real product surfaces that proves:

- a pending LIMIT whose price is hit only on an intermediate crossed candle fills through the visible `+5` action at the earliest correct candle;
- order/execution/position/cash/T+2/marker state is exact and the execution remains exactly once after rewind, forward and reload;
- a deliberately mismatched checklist date sent to the backend is rejected and creates no row;
- the correct current-context checklist still saves, appears in the in-workspace Journal and reloads.

Do not classify the mismatched-date rejection as an unexplained runtime error. Contain and assert only the deliberately generated request, as done for the existing T+2 rejection; all other page, console, provider and indicator-request errors remain blocking.

## Required verification and handoff

Run and record:

```bash
git diff --check
cd backend && ../.venv/bin/python -m pytest app/tests/test_practice_workflow.py app/tests/test_trade_lifecycle.py app/tests/test_api_integration.py -q
cd backend && ../.venv/bin/python -m pytest -q
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

Review fresh 1440×1000 and 1280×800 screenshots and retain machine-readable results. Compare final IDs against the 246-ID baseline and report missing, duplicates, renamed IDs, changed pass values, changed blocking classification and additive closure IDs.

Update the Batch 4 ExecPlan progress, decisions, deviations and final evidence. Handoff must list B4-R01–B4-R03 separately, exact commands/counts/artifact paths/error arrays/DB hash/HEAD/tag, and any limitation. Do not self-approve. Stop at Reviewer gate without starting Batch 5 or performing any git operation prohibited above.
