# Sumi ExecPlan standard

Use an ExecPlan for every development batch that changes product behavior, architecture, persistence, or dependencies. Store active plans in `docs/exec-plans/` and keep them updated during execution.

## Required structure

```markdown
# <Batch ID> — <Outcome>

## Outcome
Describe the user-visible result. Do not describe only code activity.

## Context and problem
Link the review finding, architecture decision, and acceptance IDs addressed.

## In scope
- Complete vertical capabilities included in this batch.

## Out of scope
- Explicit boundaries preventing scope drift.

## Invariants
- No-future-leak, persistence compatibility, local-first constraints, and other risks.

## Current architecture
List exact files/modules and current behavior relevant to the batch.

## Target design
Describe ownership, data flow, provider boundaries, state model, and migration behavior.

## Milestones
1. A testable milestone with exit criteria.
2. A testable milestone with exit criteria.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |

## Verification commands
List exact commands. Include `./scripts/run-product-uat.sh` for user-facing chart work.

## Rollback and compatibility
Explain how to disable/revert the batch and how persisted state is preserved or migrated.

## Risks and mitigations
Record technical, UX, dependency, licensing, and data risks.

## Progress log
- YYYY-MM-DD: concrete progress and remaining work.

## Decision log
- Decision, alternatives considered, evidence, and consequence.

## Completion evidence
- Test counts, UAT result path, screenshots, remaining known limitations.
```

## Planning rules

- One batch should normally represent 3–10 working days of coherent product work.
- Milestones must be independently verifiable; “refactor code” is not an outcome.
- Every user-facing claim maps to an acceptance ID.
- If implementation evidence contradicts the plan, update the decision log before continuing.
- If a provider spike fails its exit criteria, stop and record the result; do not quietly build around the failure.
- Do not mark a batch complete with P0/P1 failures in its scope.
