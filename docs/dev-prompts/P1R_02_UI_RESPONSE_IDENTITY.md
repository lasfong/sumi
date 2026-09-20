# DEV P1R-02 — UI request and response identity

Status: **DEV COMPLETED; REVIEW FOUND P1R-02A REWORK**. This file is historical evidence; do not rerun it.

## Read first

1. `AGENTS.md`
2. “Reproduced blocker A” in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`
3. P.4 cases `UI concurrency`/`Output` and P.5 `P1-OR-10`/`P1-OR-12` in `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`

## Exact allowed files

- `frontend/src/components/signals/SignalInspector.tsx`
- `frontend/src/components/signals/__tests__/SignalInspector.test.tsx`
- `frontend/src/types/signals.ts`
- `frontend/src/components/replay/ReplayWorkspace.tsx`: add only `currentTimestamp={currentCandle?.timestamp}` to the existing `SignalInspector` mount. Pre-edit SHA-256: `76089D000AAFFD9373781AF31313BB1A2DC91AB8A623D6E8E9C2FB607FFC658E`.
- Append evidence only to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected, including API/backend, UAT scripts, dependencies, Git index, and database.

## Required behavior

1. Give every request attempt a monotonically increasing generation/token. Invalidate it on cleanup, unmount, or invalid/no-session/unsupported context. A late success or error may change no visible state, including after ABA `index 3 → 4 → 3`.
2. On every new context/request, hide the prior point/hash/error and show loading. Accept a response only when it matches the captured session ID, index, timeframe, signal name/version, resolved `period`/`multiplier`, a nonempty hash, and exactly one point for the requested index. Reject future points, mismatched availability index/event, and mismatched timestamp when `currentTimestamp` is supplied. Remove the last-point fallback.
3. Render the accepted response's `resolved_params.period`, point `threshold`, and hash—not local input values—as result evidence. Remove frontend `COMPLETE`; align parameter-schema property names with backend `minimum`/`exclusiveMinimum`/`maximum`, and allow enum string values in the response type.
4. Do not truncate period input: only an integer 1–252 is valid. Multiplier accepts any finite value `> 0` and `<= 100`, including `0.001`; zero, empty, NaN, and Infinity are invalid and must not call the API.

## Required tests

- ABA: old index-3 response arrives during the new index-3 request; it neither renders nor ends loading. Only the new response renders.
- A late rejected promise cannot replace a newer success or show an error.
- Mismatched session/index/timeframe/version/resolved params, missing or non-unique exact point, future point, bad availability, and mismatched timestamp are rejected with no stale result.
- Returned period/threshold/hash are rendered from the accepted server response; no last-point fallback.
- Period `1.5` and multiplier `0` do not call the API; multiplier `0.001` does.

Preserve existing tests. Run the focused component test, full frontend tests, lint, build, `scripts/verify-v2.ps1`, then `scripts/run-comprehensive-uat.ps1` and `scripts/run-product-uat.ps1` sequentially. Do not edit a failing harness or assertion. Retain and review the 1440×1000 Signal Inspector screenshot. Verify the production DB hash and WAL/SHM state before/after.

Report changed files, exact test/UAT results, screenshot path, DB evidence, and deviations. End exactly:

`P1R-02 DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`
