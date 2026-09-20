# DEV P1R-02B — Strict daily timestamp validation

Status: **DEV COMPLETED; REVIEW ACCEPTED**. Historical evidence only; do not rerun.

## Read first

1. `AGENTS.md`
2. The P1R-02A review addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`

## Exact allowed files

- `frontend/src/components/signals/SignalInspector.tsx`
- `frontend/src/components/signals/__tests__/SignalInspector.test.tsx`
- Append evidence only to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. Do not change the shared `date.ts` utility, backend, types, ReplayWorkspace, API, UAT scripts, dependencies, Git/index, database, or provider data.

## Required correction

Fix only the timestamp boundary. The current helper treats any non-empty string with a matching prefix as parseable.

1. Add a small local strict daily-date parser. It must validate a real `YYYY-MM-DD` calendar date and, when a time suffix exists, validate the entire `YYYY-MM-DD[T or space]time` string. Preserve the leading Vietnam market date; do not shift it through timezone conversion.
2. Accept valid date-only, ISO datetime, timezone-bearing ISO datetime, and the existing valid space-separated backend datetime. Reject impossible dates and malformed suffixes such as `2026-02-30`, `2026-01-06Tnot-a-time`, and arbitrary text.
3. `currentTimestamp === undefined` remains the supported “not supplied” case. A defined but empty, whitespace-only, or malformed `currentTimestamp` must show a controlled validation error immediately, make no signal API call, render no result, and show no loading spinner.
4. When a valid current timestamp is supplied, malformed `point.timestamp` or `available_at_timestamp` must settle through `failValidation`: visible `signal-error`, no result, no loading.
5. Preserve the accepted P1R-02A occurrence-token, exact multiplier, ABA, late-promise, and settled-rejection logic. Do not refactor rendering.

## Required tests

- Reject `point.timestamp="2026-01-06Tnot-a-time"` even though its date prefix matches.
- Reject malformed `available_at_timestamp` the same way.
- Reject impossible calendar date `2026-02-30`.
- Defined whitespace/malformed `currentTimestamp`: controlled validation error, zero API calls, no loading/result.
- Accept and match valid `2026-01-06`, `2026-01-06T15:30:00Z`, a valid offset ISO timestamp, and `2026-01-06 15:30:00`.
- Keep all existing 15 SignalInspector tests unchanged in meaning.

Run the focused component test, full frontend tests, lint, build, `scripts/verify-v2.ps1`, comprehensive UAT, and product UAT. Do not change a failing harness. Verify the retained 1440×1000 Signal Inspector screenshot and production DB hash/WAL/SHM.

Report exact files, counts, DB evidence, and deviations. End exactly:

`P1R-02B DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`
