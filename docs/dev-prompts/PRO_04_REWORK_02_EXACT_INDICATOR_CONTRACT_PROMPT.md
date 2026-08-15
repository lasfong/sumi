# PRO-04 REWORK-02 standalone DEV authority — exact indicator output contract

You are the implementation session for exactly one bounded correction: PRO-04 REWORK-02. Do not rely on chat history.

## Mandatory read order

1. `AGENTS.md`
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
5. `docs/SESSION_HANDOFF_PROTOCOL.md`
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, especially PRO-IND-01..11 and PRO-04
7. `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`
8. `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
9. `docs/reviews/PRO_04_REVIEW_2026-08-13_R2.md`
10. `PLANS.md`

This prompt authorizes only REWORK-02. Do not commit, push, tag, release, add a dependency, mutate `backend/sumi.db`, change formulas, or start PRO-05.

## Required outcome

R1 shared-pane correction is retained. Close the remaining R2 P1 semantic contract:

1. Replace every parameter-permissive prefix/first-column fallback in `IndicatorRenderRegistry` with exact matching against the pinned backend output column contract derived from the requesting instance parameters. This includes EMA, RSI, MACD line/signal/histogram, CCI, SMA, Bollinger upper/middle/lower, ATR, and Volume SMA.
2. Missing, duplicate, parameter-mismatched, or ambiguous expected output must fail closed: no mislabeled rendered series and a truthful runtime state. A correct matching column must still be selected regardless of irrelevant column order.
3. Add deterministic regression tests for every released backend definition. Include mismatched length, mismatched Bollinger standard deviation, and mismatched MACD fast/slow/signal; each must return no rendered series. Include irrelevant preceding columns plus one exact expected output; it must render only that expected value.
4. Strengthen deterministic Product UAT to assert values/series belong to the requested parameter set, not only that values are finite. Preserve the shared-pane DOM/native-order assertion. Capture a 1280×800 viewport artifact visibly containing the shared Volume/Volume SMA pane and ATR pane (scroll when necessary) as well as the 1440×1000 view.
5. Consolidate the duplicated prose in `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md` into one coherent, historically complete plan. Do not delete or weaken evidence.

## Required verification and handoff

- Record `backend/sumi.db` SHA-256 before/after; it must stay `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`.
- Run focused backend/frontend tests, `scripts/verify-v2.ps1`, deterministic `scripts/run-product-uat.ps1`, and `scripts/verify-product.sh` through the repository-supported Windows-compatible invocation when available.
- Retain UAT results, manifest reconciliation, runtime/request classification, cleanup evidence, and both required screenshots with hashes.
- Update the ledger and ExecPlan with exact counts, commands, artifacts, and deviations. Stop only at `IMPLEMENTED — REVIEW PENDING`; do not self-approve or start PRO-05.
