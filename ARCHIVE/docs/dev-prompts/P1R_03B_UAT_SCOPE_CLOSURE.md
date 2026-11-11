# DEV P1R-03B — Phase 1 UAT and scope closure

Status: **AUTHORIZED NEXT ONLY**. Stop afterward; Phase 2 remains blocked.

Read `AGENTS.md`, `docs/reviews/P1R_03_SCOPE_INVENTORY.md`, and the latest P1R-03A addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`.

## Exact allowed files

- Only the Phase 1 Signal Inspector block in `scripts/product-uat.mjs`
- Correct `docs/reviews/P1R_03_SCOPE_INVENTORY.md`
- Append evidence to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. Do not change unrelated UAT hunks, product code, tests, prompts, Git/index, baseline artifacts, database, or provider data. Do not clean/restore/commit/stash anything.

## Required corrections

1. Inventory counts must distinguish the P1R-03A pre-report snapshot (`80` untracked, `27` new), its post-report state (`81` / `28`: 15 code + 13 docs), and the P1R-03B start state after this authorization prompt (`82` / `29`: 15 code + 14 docs). Classify `AGENTS.md` and the two Doraemon/plan edits as `KEEP_SEPARATE` reviewer governance, never `KEEP_AS_P1` product code.
2. Add an exact Phase 1 include manifest and a separate-drift manifest. Keep hunk-level separation for `ReplayWorkspace.tsx` and `product-uat.mjs`; do not claim the separate drift is physically cleaned or accepted.
3. Harden only the Signal Inspector UAT block:
   - Registry must contain exactly the expected `volume.spike` v1.0.0 contract used by the test.
   - Select exactly one API point with `bar_index === observed_current_index`; fail on duplicates, missing point, or any future point. Never use `.at(-1)` as identity.
   - Require `VALID` exactly after restoring normal parameters. `COMPLETE` is forbidden.
   - Prove browser states `INSUFFICIENT_HISTORY`, spike `true`, spike `false`, and a controlled API error. For each success state, compare visible UI with the exact matching API response.
   - Inject the controlled error only for the UI calculation request using a narrowly scoped Playwright route, assert `signal-error` visible and loading/results absent, then always remove the route in `finally` and restore period `20`, multiplier `2.0`.
   - Replace fixed `400ms` waits with response/state-based waits tied to the exact requested parameters. End with a restored valid state and retain the 1440×1000 screenshot.
   - Assert the API response contains no point beyond `observed_current_index`. Do not create Trade/P&L state.
4. Do not weaken or delete any existing assertion. Record any inability to produce true/false with the deterministic UAT dataset as a blocker; do not fake expected values.

Run syntax check for the UAT script, focused SignalInspector tests, `scripts/verify-v2.ps1`, comprehensive UAT, and product UAT. Check final DB hash/WAL/SHM and review the retained screenshot.

End exactly:

`P1R-03B DONE; UNRELATED DRIFT NOT CLEANED; PHASE 2 NOT STARTED`
