# DEV P1R-01A — Signal Snapshot Integrity

Status: **AUTHORIZED NEXT REWORK ONLY**  
This is a narrow correction to P1R-01. Stop and return to reviewer before P1R-02.

## Read first

1. `AGENTS.md`
2. `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`, including the P1R-01 review addendum
3. P.4–P.5 in `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`

## Exact allowed files

- `backend/app/domain/signals/models.py`
- `backend/app/domain/strategy/signal_binding.py`
- `backend/app/tests/test_signals.py`
- `backend/app/tests/test_signal_binding.py`
- Append evidence to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Every other file is protected. Do not touch frontend, API/schema/service/registry, execution code, Git index, or `backend/sumi.db`.

## Required behavior

1. A referenced `VALID` signal snapshot must match its registry output type:
   - bool signal: value is exactly `bool`;
   - float signal: value is a finite `int`/`float`, never bool;
   - `output_type` matches the registry definition.
   Malformed values fail closed with `value=None`, `is_valid=False`, quality `INVALID_VOLUME`, and a descriptive reason. Do not enter the evaluator.
2. For each signal alias used by `cross_up` / `cross_down`:
   - previous point must be exactly `current.bar_index - 1`;
   - each point must have `available_at_index == bar_index` and `availability_event == "BAR_CLOSE"`;
   - missing, future, same-bar, non-adjacent, unavailable, or malformed previous points fail closed. Use `INSUFFICIENT_HISTORY` for missing/non-adjacent previous state; do not invent a new quality enum.
3. `SignalOutputPoint.to_dict()` must never silently emit `quality="VALID"` with a null value caused by sanitizing NaN/Infinity or a wrong runtime type. Reject an internally invalid `VALID` point explicitly. Existing valid bool/float serialization and non-VALID null serialization must remain unchanged.

## Required tests

- Update valid cross fixtures to current bar 5 and previous bar 4 with correct availability metadata.
- Same-bar previous point is fail-closed.
- Future and non-adjacent previous points are fail-closed.
- Current or previous float snapshot with NaN/Infinity is fail-closed before evaluation.
- Float signal with bool/string value and bool signal with numeric/string value are fail-closed.
- Mismatched `output_type` and invalid availability metadata are fail-closed.
- Direct serialization of a malformed `VALID` point raises instead of producing `VALID + null`.
- Existing 24 focused tests remain green; run the full backend suite in a temporary DB.

## Stop conditions and handoff

Do not change `rule_evaluator.py` or add a dependency. If the behavior cannot be implemented within the allowed files, stop and report why.

Report changed files, focused/full test counts, DB hash/WAL/SHM, and deviations. End with:

`P1R-01A DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`.

