# DEV P1R-01B — Numeric and serialization boundary

Status: **DEV COMPLETED; REVIEW ACCEPTED 2026-09-13**. This file is historical evidence; do not rerun it.

## Read first

1. `AGENTS.md`
2. The P1R-01A review addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`

## Exact allowed files

- `backend/app/domain/signals/models.py`
- `backend/app/domain/strategy/signal_binding.py`
- `backend/app/tests/test_signals.py`
- `backend/app/tests/test_signal_binding.py`
- Append evidence only to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

Everything else is protected. Do not touch the Git index, database, API, registry, frontend, evaluator, dependencies, or Doraemon/provider data.

## Required corrections

1. A float snapshot value is valid only when it is an `int`/`float` other than `bool` **and can be represented as a finite Python float**. An arbitrarily large integer must never raise `OverflowError`:
   - binding returns nullable fail-closed `INVALID_VOLUME` before evaluation;
   - direct serialization raises an explicit `ValueError`.
2. `SignalOutputPoint.to_dict()` accepts only known output types:
   - `bool` → exact bool;
   - `float` → finite representable number;
   - `enum` → string.
   Unknown output types and mismatched `VALID` values raise `ValueError`.
3. Numeric explanation fields (`baseline`, `current_volume`, `relative_volume`, `threshold`) must never throw or emit a non-finite number when given an oversized integer; sanitize such a field to `None` under the existing policy.

Use one small shared conversion/validation helper where practical. Do not broaden the task.

## Required tests and verification

- Add tests for current and previous float snapshots containing `10**10000`: both fail closed without entering the evaluator.
- Add direct serialization tests: oversized float value raises `ValueError`; oversized explanation field becomes `None`; enum string succeeds; enum bool/number and unknown output type raise `ValueError`.
- Preserve all existing 29 focused tests. Run focused tests and the full backend suite with a temporary SQLite database; verify the production DB hash and WAL/SHM state are unchanged.

Report changed files, counts, DB evidence, and deviations. End exactly:

`P1R-01B DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`
