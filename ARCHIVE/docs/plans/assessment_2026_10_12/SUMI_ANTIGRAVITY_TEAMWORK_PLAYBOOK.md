# SUMI Antigravity Teamwork Playbook — 2026-10-12 (PROPOSED)

For a Gemini Flash-powered implementer team under a single orchestrator/reviewer chain. Preserves AGENTS.md gates; optimizes for cheap parallel reads + single-writer correctness.

## Roles
- **Orchestrator (1):** owns wave order, batch dispatch, file ownership, STOP enforcement, evidence ledger, GPT-6/architect escalation. Never edits implementation files concurrently with DEV. Persists `STATE.json`-style progress per program (separate from dev-program file).
- **Investigators (parallel, read-only):** one per wave-front (data-safety / replay-quant / FE-integration / contracts-UX / quality). Bounded file lists, VERIFIED/REPORTED/INFERRED/UNKNOWN labels, file:line evidence. Must not edit shared files.
- **Implementer (1 per batch = single writer):** owns the batch tree slice; ExecPlan first; vertical slice only; records deviations in decision log.
- **Test/evidence agent:** owns temp-DB runs, golden/chaos/snapshot suites, UAT capture (1440×1000 + 1280×800, machine-readable + screenshots). Never weakens tests without reviewer approval.
- **Reviewer (independent context):** inspects diff vs ExecPlan + acceptance IDs + evidence; sole acceptor. Separation: reviewer never authored the batch.

## File ownership per batch
Orchestrator publishes the allowed-file list from DEV_BATCHES + prompt spec before dispatch (e.g., B01: dependencies/db/workflows only). Implementer touches nothing else; cross-scope need → STOP + architect decision. Reviewer/test agents read-only except evidence outputs in their own paths.

## Parallel vs sequential
- Parallel: investigator sweeps on disjoint areas; test-agent evidence prep (golden vectors, UAT scripts) during implementation; B31 vs B32-disjoint-files; contract-spec vs UX-spec drafting.
- Sequential: one DEV writer at a time on prod paths; W0 strict; B20→B21→B22; B10→B11; B30→B31; B40→B41. Flash agents excel at bounded parallel reads — keep writes serial.

## Architect-approval gates (do not proceed on implementer authority)
Provider spike failure; persistence-migration data-loss risk; new dependency (license/security); backend-contract change outside scope; acceptance-criteria inconsistency/infeasibility; unexplained PnL parity delta; synthetic-data policy exception; any prod-data/storage operation (owner-only sempre). Trivial implementation details: no approval needed.

## Reporting format (per batch)
`Outcome · scope delta vs ExecPlan · acceptance table (ID → code evidence → test/UAT evidence) · verification commands + paths · screenshots · known limitations · decision log · rollback note.` Evidence retained even on failure (G-03).

## Git/commit policy
One batch = one clean commit (no unrelated files); ExecPlan + evidence referenced; attribution per repo reminder; push/PR only on owner instruction. Tree must be clean before next dispatch. Never commit secrets, raw env, or prod DB artifacts.

## Failure escalation
Red test/UAT → STOP, record, return to orchestrator; no workarounds (no UX masking domain, no weakened asserts, no skipped predecessors). Data-loss-risk or policy gray zone → architect + owner. Deployment/redeploy/rollback/prod-console/R2/storage ops → owner performs from agent-prepared instructions (workspace rules).

## Batch acceptance gate (all required)
1. Acceptance IDs mapped with code + test/UAT evidence. 2. Unit/integration + lint/build green; touched Replay/Chart/Indicator/Drawing has green browser UAT + reviewed screenshots. 3. Temp-DB only; prod hash unchanged where applicable. 4. No unrelated diff. 5. ExecPlan progress/decisions/evidence updated. 6. Independent reviewer sign-off. 7. STOP conditions re-checked (deps accepted, no scope contract drift).

## Flash-efficiency notes
Dispatch with explicit file lists + acceptance IDs + forbidden zones; cap investigator scope (~12 files); require labels + line refs; forbid runtime/DB/prod ops in prompt; reuse P-lineage engines as inputs, not re-derivations.
