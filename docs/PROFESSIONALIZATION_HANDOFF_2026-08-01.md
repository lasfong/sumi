# Sumi Professionalization cross-machine handoff

> **SUPERSEDED (2026-08-09).** This handoff describes the pre-PRO-00 state and is retained as historical provenance only. For current work, read `AGENTS.md`, `docs/INDEX.md`, `docs/AUTONOMOUS_EXECUTION_STATE.md`, the active PRO dossier, and the active DEV prompt named by the ledger.

## Purpose

This document is the concise operational handoff for continuing the Professionalization program on another machine. It supplements, but does not replace, `AGENTS.md`, the canonical V3 documents, the Professionalization master plan, or the active batch ExecPlan.

## Repository state

- Remote: `https://github.com/lasfong/sumi.git`
- Active branch: `master`
- PRO-00 implementation commit: `55ec5f9` — `fix(replay): close PRO-00 integrity and evidence gaps`
- Current authorized batch: PRO-00 verification and independent review only
- PRO-01 through PRO-12: not started and not authorized
- Release status: no Professional, product-complete, release-ready, or “TradingView-like” claim is authorized

The program has 13 ordered batches: PRO-00 through PRO-12. PRO-11 is conditional on PRO-10 approving a market-data provider.

## Current PRO-00 status

Completed and pushed in the implementation commit:

- backend-authoritative `blind_practice` and `signal_review` intent;
- sanitized, versioned Scanner replay source context across create/list/get/navigation/WebSocket paths;
- exact actual-candle reveal boundary and fail-closed legacy/malformed handling;
- frontend removal of raw scanner payload as display authority;
- honest Scanner/Replay labels, reveal-only context, marker, rewind, and reload behavior;
- checked-in 275-assertion UAT manifest and strict reconciliation/failure retention;
- focused and full technical checks plus retained successful browser evidence.

Still pending:

- a clean-machine rerun of the exact canonical standalone/full product gates;
- final evidence reconciliation and ExecPlan completion update;
- independent Reviewer inspection and decision.

No new PRO-00 feature work should occur unless a verification failure proves a defect inside PRO-00 scope.

## Clone and provenance check

```bash
git clone https://github.com/lasfong/sumi.git
cd sumi
git checkout master
git pull --ff-only origin master
git status --short --branch
git log -1 --oneline --decorate
```

Before work, confirm that the checkout contains commit `55ec5f9` or a documented later commit containing it. Do not reset, clean, restore, or overwrite unexpected local changes.

## Required reading order on the new machine

1. `AGENTS.md`
2. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
5. `docs/DEVELOPMENT_OPERATING_MODEL.md`
6. `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
7. `PLANS.md`
8. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
9. `docs/dev-prompts/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE_PROMPT.md`
10. `docs/exec-plans/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE.md`
11. `docs/dev-prompts/PRO_00_VERIFICATION_CONTINUATION_PROMPT.md`

Reports under `docs/tester/` are research inputs, not acceptance or release authority.

## How to continue

Open the repository root in a fresh Codex task and issue:

```text
Execute docs/dev-prompts/PRO_00_VERIFICATION_CONTINUATION_PROMPT.md exactly. Continue PRO-00 verification only from the current origin/master checkout. Do not start PRO-01. Do not modify product code unless a verification failure proves an in-scope PRO-00 defect. Update the PRO-00 ExecPlan and evidence, then stop at the independent Reviewer gate.
```

If a verification command appears stalled, inspect its owned processes and logs, enforce a bounded timeout, retain partial evidence, and stop only the exact processes started by that run. Never leave a test runner active indefinitely.

## After PRO-00 review

PRO-01 may be planned only after the PRO-00 ExecPlan contains an independent Reviewer approval. If approval is absent, the next task must stop and report the blocker. The future PRO-01 task must create its own standalone authority and bounded ExecPlan before product code and must not begin PRO-02.

## Git policy

- Verification does not imply authorization to commit or push.
- A corrective commit requires explicit user authorization and must contain only the bounded PRO-00 fix/evidence changes.
- Preserve `backend/sumi.db` byte-for-byte; automated verification must use a temporary database.
- Do not move, retag, or rewrite `v2.0.0-rc2`.
