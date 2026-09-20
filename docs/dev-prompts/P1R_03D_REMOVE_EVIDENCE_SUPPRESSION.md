# DEV P1R-03D — Remove UAT evidence suppression

Status: **AUTHORIZED NEXT ONLY**. This is the final narrow harness correction. Do not change product code or start Phase 2.

Read the P1R-03C review addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md` and inspect only the Phase 1 block in `scripts/product-uat.mjs`.

## Allowed files

- Only lines inside the Phase 1 Signal Inspector block in `scripts/product-uat.mjs`
- `docs/reviews/P1R_03_SCOPE_INVENTORY.md`
- Append evidence to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. No product/test/helper/listener/manifest changes; no Git/index/database/cleanup actions.

## Make exactly this correction

1. Delete both `page.evaluate` blocks that replace/restore `window.console.error`. Do not replace them with any console, network, listener, or evidence filtering.
2. Extend the local `fetchApiSignalPoint` return value with `timeframe: data.timeframe`. Replace `route.abort` with one deterministic `route.fulfill` response for the exact `period=20`, `multiplier=2.1` request. Return HTTP 200 JSON `{ session_id: sessionId, observed_current_index: initialData.observedIndex, timeframe: initialData.timeframe, results: [] }`. This must drive the existing UI response-contract validation to `signal-error` because `volume.spike` is missing. Do not modify product code.
3. Configure `NegativeOperationTracker` with the exact calculate endpoint and `expectedStatus: 200`; remove `allowNoResponses`. Require `pass === true`, `capturedResponseCount === 1`, and the captured URL/status to match.
4. Capture the injected request body in the route handler and persist under `hardening.signalControlledError`: tracker snapshot, injected request, injected response, and `signalRequestFailureDelta: 0`. Assert the route fired once and no new request failure for this exact Signal calculate endpoint occurred. Keep every global evidence collection untouched.
5. Restore `20`/`2.0` through the exact-response helper and retain all existing state/UI/API assertions and screenshot. Recalculate the final Phase 1 hunk range/count and inventory. Record P1R-03D start as `84` untracked total / `31` post-freeze: 15 code + 16 docs.

Verification is intentionally bounded: run `node --check scripts/product-uat.mjs`, the focused 17 SignalInspector tests, and `scripts/run-product-uat.ps1`. Do not rerun `verify-v2` or comprehensive UAT because product code is unchanged and their immediately preceding P1R-03C results remain applicable. Verify the new artifact fields, zero runtime errors, reconciliation, DB hash/WAL/SHM, and screenshot.

End exactly:

`P1R-03D DONE; PHASE 1 AWAITS REVIEWER SEAL; PHASE 2 NOT STARTED`
