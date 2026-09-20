# DEV P1R-02A — Render identity and settled rejection

Status: **DEV COMPLETED; REVIEW FOUND P1R-02B TIMESTAMP REWORK**. Historical evidence only; do not rerun.

## Read first

1. `AGENTS.md`
2. The P1R-02 review addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`

## Exact allowed files

- `frontend/src/components/signals/SignalInspector.tsx`
- `frontend/src/components/signals/__tests__/SignalInspector.test.tsx`
- Append evidence only to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. Do not modify backend, types, ReplayWorkspace, API, UAT scripts, dependencies, Git index, database, or provider data.

## Required corrections

1. A completed result/error belongs to one **request occurrence**, not merely a reusable value key. Add a render identity that changes whenever `currentKey` changes, including `A → B → A`, and store that identity with accepted/error state. Keep the monotonic request generation for late-promise rejection. Do not synchronously set state in an effect just to clear UI; derive visibility from occurrence identity so lint remains green.
2. Remove all epsilon comparisons. Configuration identity and returned resolved parameters must match exactly. Changing multiplier `2` to `2.0000005` immediately hides the old result and starts loading.
3. Every current-generation response mismatch must settle as a visible controlled error and no result—never an endless loading spinner. Use one helper so all validation exits behave consistently.
4. When `currentTimestamp` is supplied, missing/empty point timestamp is a mismatch. `point.timestamp` and `available_at_timestamp` must both represent the requested daily date. Preserve accepted ISO/date normalization only when both values are present and parseable.

A small stable occurrence token such as `useMemo(() => Symbol(currentKey ?? 'invalid'), [currentKey])`, stored in accepted/error state, is an acceptable implementation. Do not refactor unrelated rendering.

## Required tests

- Completed A → pending B → pending new A: completed A is hidden and loading remains until new A resolves.
- Multiplier `2 → 2.0000005`: old result is hidden; a response resolved as `2` is rejected; exact `2.0000005` is accepted.
- Prior A error → B → new A: prior error is hidden and new A shows loading.
- Make every mismatched-response case await the controlled error before asserting no result; no test may pass from the initial empty render.
- Empty point timestamp and mismatched `available_at_timestamp` are rejected when `currentTimestamp` is supplied.
- Preserve the existing in-flight ABA and late-rejection tests.

Run focused and full frontend tests, lint, build, `scripts/verify-v2.ps1`, comprehensive UAT, then product UAT. Do not modify a failing harness. Review the retained 1440×1000 Signal Inspector screenshot and verify production DB hash/WAL/SHM before and after.

Report exact files, results, screenshot, DB evidence, and deviations. End exactly:

`P1R-02A DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`
