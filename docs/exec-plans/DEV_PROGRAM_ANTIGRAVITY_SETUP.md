# DEV-GOV-01 — Autonomous Antigravity handoff

## Outcome

Owner can launch one repository brief; the DEV orchestrator runs bounded implement/review/fix cycles without manual message relay.

## Context and problem

Phase 1 has a reviewer seal, but historical single-batch prompts force expensive manual handoffs. This is orchestration preparation, not Phase 2 implementation.

## In scope

Program entrypoint, routing/state, read-only validation helper, Antigravity skill, superseding execution-governance notices.

## Out of scope

Product code, DB access/migration, cleanup/staging/commits, provider acquisition, IDE installation, paid subscriptions or actual autonomous product execution.

## Invariants

Preserve source checkout/index/database; retain all product/data/acceptance gates. No self-review disguised as independent review; no claim all phases are unblocked.

## Current architecture

AGENTS.md and the final roadmap govern bounded work. docs/dev-prompts holds historical Phase 1 instructions. Existing dirty baseline is broader than HEAD and the accepted P1 files.

## Target design

START.md owns the loop; ROADMAP.md routes task cards; STATE.json persists transitions. One writer, independent reviewers, scoped repair retries, continue unrelated unblocked paths. Native Teamwork preferred where available; a skill is an entrypoint, not an agent scheduler.

## Milestones

1. Write compact program instructions and initial state, keeping Phase 1 accepted and bootstrap pending.
2. Validate state dependencies, references, negative cases and protected-file invariants.

## Acceptance mapping

| ID | Implementation | Verification |
| --- | --- | --- |
| GOV-01 | Single brief + skill | File/readability inspection |
| GOV-02 | Autonomous bounded loop, independent reviewer and owner gates | Instruction review |
| GOV-03 | Checkpoint/DAG validator | Valid initial state + invalid-state self-tests |
| GOV-04 | No product/index/DB edits | Incremental scope and hash inspection |

## Verification commands

`node --check scripts/verify-dev-program.mjs`

`node scripts/verify-dev-program.mjs --self-test`

`node scripts/verify-dev-program.mjs`

Skill frontmatter validator when Python/PyYAML are available. No product suites required for documentation/read-only tooling setup; live Antigravity execution is not tested here.

## Rollback and compatibility

Revert only this setup's new files/notices if needed. Preserve historical prompts and their evidence. No runtime product dependency introduced.

## Risks and mitigations

Model/runtime limits cannot be removed by prompts. Bootstrap checks actual reviewer capability and persists resume state. Dirty baseline cannot be recreated by HEAD-only checkout. Real data and owner decisions can block completion; never manufacture them.

## Progress log

- 2026-09-15: Prepared autonomous program configuration; verification recorded below after execution.

## Decision log

- Prefer skills over new legacy workflows; native command verified as /teamwork-preview, not /team. Availability differs by runtime/account.
- Keep source checkout unchanged. Native isolated workspace requires explicit destination/copy scope at launch, excluding private data/secrets.
- Supersede historical handoff governance, not product semantics or evidence requirements.

## Completion evidence

- 2026-09-15: Node syntax check PASS; structural checkpoint validation PASS, dependency-ready queue contains only BOOTSTRAP; self-tests PASS (13 cases, including cycles, unknown dependencies, missing evidence, premature completion and valid completion). Canonical roadmap task-card coverage is checked by the validator.
- Changed tracked-document whitespace check PASS. Reviewed entrypoint/skill references, authorization precedence, scoped repair/escalation and native-runtime limitations. Root README and historical prompt/roadmap notices route to the new entrypoint.
- Production DB SHA-256 remains `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`; WAL/SHM absent. Staged diff summary remains 159 files / 26429 deletions. No Git index operation or product source edit performed.
- Bundled skill Python validator could not run: repository virtualenv executable access denied; system Python launcher reports no installed Python. Frontmatter and skill references were inspected manually. This is a disclosed tooling limitation, not a passed Python validation.
- No Phase 2 code implemented; no product tests or live Antigravity execution claimed. Program is READY_FOR_BOOTSTRAP, not production-ready. Native model/independent-review availability and runnable product baseline are checked at first launch.
