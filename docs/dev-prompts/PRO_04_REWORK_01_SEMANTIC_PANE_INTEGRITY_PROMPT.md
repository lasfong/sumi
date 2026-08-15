# PRO-04 REWORK-01 standalone DEV authority — semantic mapping and shared-pane integrity

You are the implementation session for exactly one bounded correction: PRO-04 REWORK-01. Do not rely on chat history.

## Mandatory read order

1. `AGENTS.md`
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
5. `docs/SESSION_HANDOFF_PROTOCOL.md`
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, especially PRO-IND-01..11 and PRO-04
7. `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`
8. `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
9. `docs/reviews/PRO_04_REVIEW_2026-08-13_R1.md`
10. `PLANS.md`

This prompt authorizes only REWORK-01. Do not commit, push, tag, release, add a dependency, mutate `backend/sumi.db`, change accepted indicator formulas, or start PRO-05.

## Required outcome

Correct both P1 findings from R1 without expanding scope:

1. **Explicit semantic adapters.** Replace `available[0]` / arbitrary first-column mapping with an exhaustive released-definition adapter. Each released definition must select only its expected pinned backend output columns based on its parameters. In particular map SMA, ATR, and Volume SMA to their exact outputs; map Bollinger upper/middle/lower semantically; retain correct EMA/RSI/MACD/CCI behavior. Missing, unknown, or ambiguous columns must fail closed and never render under a released label.
2. **One shared volume chrome group.** Group pane chrome by physical `paneId`, not instance. Raw Volume and Volume SMA must share exactly one `volume` pane chrome group while retaining discoverable, usable controls and values for each instance. ATR must appear in its own correctly aligned oscillator pane. Preserve ordering, settings, visibility, removal, persistence, replay navigation, and remount behavior.
3. **Regression evidence.** Add focused tests that inject irrelevant/prepending response columns and prove semantic adapters ignore/reject them. Add DOM/pane tests proving exactly one volume chrome group, correct association of raw-volume histogram and average-volume line, and no ATR/Volume label shift. Extend UAT to test semantic values plus native-pane/chrome alignment through add, hide, remove, reload, replay navigation, and remount. Do not weaken, rename, duplicate, or make non-blocking existing acceptance assertions.

## Required verification and handoff

- Record production DB SHA-256 before/after; it must remain `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`.
- Run focused backend/frontend tests, `scripts/verify-v2.ps1`, deterministic `scripts/run-product-uat.ps1`, and `scripts/verify-product.sh` through a Windows-compatible repository invocation when available. Retain machine-readable results, manifest reconciliation, cleanup evidence, and new 1440×1000 plus 1280×800 screenshots visibly proving the corrected volume/ATR alignment.
- Update the PRO-04 ExecPlan and state ledger with exact commands, counts, hashes, artifact paths, deviations, and the completed reviewer checklist.
- Stop only at `IMPLEMENTED — REVIEW PENDING`. Do not self-approve or start PRO-05.
