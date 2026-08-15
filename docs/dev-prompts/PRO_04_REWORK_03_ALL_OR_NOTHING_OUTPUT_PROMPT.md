# PRO-04 REWORK-03 standalone DEV authority — all-or-nothing pinned output contract

You are the implementation session for exactly one bounded correction: PRO-04 REWORK-03. Do not rely on chat history.

## Mandatory read order

1. `AGENTS.md`
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
5. `docs/SESSION_HANDOFF_PROTOCOL.md`
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, especially PRO-IND-01..11 and PRO-04
7. `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`
8. `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
9. `docs/reviews/PRO_04_REVIEW_2026-08-13_R3.md`
10. `PLANS.md`

This prompt authorizes only REWORK-03. Do not commit, push, tag, release, add a dependency, mutate `backend/sumi.db`, change formulas, or start PRO-05.

## Required outcome

Keep REWORK-01 and REWORK-02 behavior. Close the R3 P1 contract precisely:

1. Define one exact pinned backend column-name contract for each released definition/parameter set. Do not accept aliases: CCI must use the canonical current engine output `CCI_${length}_0.015`; ATR must use the actual current pinned output; Bollinger must use one exact discovered column format for its requested length/std. Retain exact contracts for EMA, RSI, MACD, SMA, and Volume SMA.
2. MACD and Bollinger must be all-or-nothing. Render only if all required exact semantic outputs exist and are unambiguous; otherwise return no series and preserve an honest warming/error state. Never expose a partial released MACD or Bollinger indicator.
3. Add focused regression tests rejecting CCI/ATR aliases, alternate Bollinger spellings, and partial MACD/Bollinger payloads. Keep mismatched-parameter and irrelevant-preceding-column evidence.
4. Strengthen Product UAT to match the visible runtime values for SMA, Bollinger upper/middle/lower, ATR, and Volume SMA to the exact expected values in the scoped backend response for the same session/index/parameters. A finite value or matching document parameter alone is insufficient.

## Required verification and handoff

- Record `backend/sumi.db` SHA-256 before/after; it must remain `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`.
- Run focused backend/frontend tests, `scripts/verify-v2.ps1`, deterministic `scripts/run-product-uat.ps1`, and `scripts/verify-product.sh` through a Windows-compatible repository invocation when available.
- Retain result JSON, manifest reconciliation, request/runtime classification, cleanup evidence, and 1440×1000 plus 1280×800 screenshots with hashes.
- Update the ExecPlan and ledger with exact commands/counts/artifacts. Stop at `IMPLEMENTED — REVIEW PENDING`; do not self-approve or start PRO-05.
