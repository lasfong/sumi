# DEV P1R-01 — Backend Contract Hardening

Status: **DEV COMPLETED; REVIEW FOUND A NARROW FOLLOW-UP (P1R-01A)**  
Model assumption: low-capability implementation model  
Stop after this batch and return to reviewer.

## Read first

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`
4. Sections P.3–P.6 of `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`
5. `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md` reviewer notice and frozen contracts

## Exact scope

Allowed production files:

- `backend/app/domain/signals/models.py`
- `backend/app/domain/signals/registry.py`
- `backend/app/domain/signals/volume.py`
- `backend/app/domain/strategy/signal_binding.py`
- `backend/app/schemas/signal_schema.py`

Allowed test files:

- `backend/app/tests/test_signals.py`
- `backend/app/tests/test_signal_binding.py`
- `backend/app/tests/test_signals_api.py`

Allowed documentation file:

- Append progress/evidence to `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`.

Every other file is protected for this batch. In particular: do not edit frontend, services, routes, `main.py`, UAT scripts, migrations, models, execution/backtest code, the Git index, or `backend/sumi.db`.

## Required fixes

1. Reject an explicit `params: null` with HTTP 422. Omitted params still resolve defaults.
2. Make registry metadata match the frozen multiplier range `> 0` and `<= 100`: use JSON-Schema-style `exclusiveMinimum: 0` and `maximum: 100`. Do not invent `0.01` or `0.1`.
3. Remove the uncontracted `COMPLETE` signal quality. For the flat resolved-parameter map, the canonical hash utility accepts only finite non-bool integers/floats and preserves the existing valid golden hash; reject bool, null, strings, containers, NaN, and Infinity.
4. Ensure all derived baseline/RVOL/value fields are finite before returning `VALID`. If finite inputs create a non-finite derived baseline or RVOL, return null value with quality `INVALID_VOLUME`, reason `NON_FINITE_DERIVED_VALUE`, and no non-finite explanation field. Never serialize NaN/Infinity.
5. Support frozen DSL `cross_up` / `cross_down` semantics without changing `rule_evaluator.py`. Preserve existing callers by adding an optional `previous_signal_points` argument after existing arguments. For every registered signal alias used directly as a cross operand, require its previous point, validate current and previous quality/value before evaluation, and bind `previous_<alias>`. A missing/invalid previous point returns nullable/fail-closed. Numeric cross operands need no previous point.

## Tests required before stopping

- `params:null` is 422; omitted params are defaults.
- Registry multiplier metadata expresses exclusive zero and agrees with validation at `0.001`, `0`, and `100`.
- Extreme finite inputs cannot produce a `VALID` point containing or hiding non-finite derived data.
- Hash utility rejects bool, NaN, and Infinity; existing golden hash remains unchanged.
- Valid `cross_up` and `cross_down` over current/previous Relative Volume points evaluate correctly.
- Missing or invalid previous signal point returns nullable/fail-closed and never enters boolean evaluation.
- All existing Phase 1 backend tests remain green in a fresh temporary database.

Run only backend focused tests first. Then run the full backend test suite. Record production DB hash and WAL/SHM before and after. Do not run browser UAT in this backend-only batch.

## Stop conditions

Stop without workaround if any fix requires an unlisted file, changing the existing evaluator, adding a dependency, inventing/changing the four frozen quality values beyond removing `COMPLETE`, touching execution/trades, or altering an unrelated test. Report the blocker and exact evidence.

## Handoff

Report only: changed files, test commands/counts, DB hash, remaining limitations, and deviations. State clearly: `P1R-01 DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`.
