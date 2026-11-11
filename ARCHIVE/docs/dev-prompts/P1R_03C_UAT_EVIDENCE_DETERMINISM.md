# DEV P1R-03C — UAT evidence determinism

Status: **AUTHORIZED NEXT ONLY**. This is a harness/evidence correction; do not change product code or start Phase 2.

Read `AGENTS.md`, the P1R-03B review addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`, and the current Phase 1 block in `scripts/product-uat.mjs`.

## Exact allowed files

- Only the Phase 1 Signal Inspector block in `scripts/product-uat.mjs`
- `docs/reviews/P1R_03_SCOPE_INVENTORY.md`
- Append evidence to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. Do not change product code/tests, shared UAT helpers/listeners/manifest, dependencies, Git/index, database, baseline artifacts, or unrelated hunks. Do not clean, restore, commit, or stash.

## Required result

1. Add one local predicate in the Phase 1 block that safely parses `request.postDataJSON()` and matches: POST, the exact session calculate path, exactly one `signals` item, `volume.spike`, `1.0.0`, and strict-equality `params.period`/`params.multiplier`. Use it for every `waitForResponse`; ignore intermediate requests caused by filling two inputs.
2. Keep the controlled request failure narrowly scoped to that same predicate with `period=20`, `multiplier=2.1`. Prove exactly one matching new `requestFailures` record was produced, retain that record, and persist the completed negative-operation snapshot under `hardening` for review. Fail if the operation snapshot does not pass.
3. Never truncate, reset, splice, filter, or otherwise erase `expectedPracticeConsoleErrors`, `runtimeErrors`, `requestFailures`, `apiOutcomes`, or any reconciliation evidence. Route cleanup remains in `finally`; restore `20`/`2.0` using the exact-response helper and verify exact `VALID` UI/API identity.
4. Record the P1R-03C start state (`83` untracked total / `30` post-freeze: 15 code + 15 docs), then recalculate the final diff. Correct every stale hunk line range and added/deleted count in `P1R_03_SCOPE_INVENTORY.md`; do not leave the current `+72` / `568–638` claim if it is no longer true. Preserve the earlier snapshot history and all `KEEP_SEPARATE` classifications.
5. Do not add or weaken assertion IDs. Do not change signal semantics or create Trade/P&L state.

Run `node --check scripts/product-uat.mjs`, the focused SignalInspector tests, `scripts/verify-v2.ps1`, comprehensive UAT, and product UAT. Inspect `results.json` for the persisted controlled-error evidence, reconciliation, runtime errors, and exact Signal request failure. Verify the production DB hash/WAL/SHM and review the 1440×1000 screenshot.

End exactly:

`P1R-03C DONE; PHASE 1 AWAITS REVIEWER SEAL; PHASE 2 NOT STARTED`
