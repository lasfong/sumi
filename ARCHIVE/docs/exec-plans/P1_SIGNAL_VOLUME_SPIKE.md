# P1-SIG-01 + P1-SIG-02 — Explainable Causal Volume Spike

> **Reviewer notice — 2026-09-12:** DEV's completion claim is rejected pending rework. `P1-OR-08`, `P1-OR-10`, cross dependency handling, non-finite derived values, and baseline scope integrity have unresolved failures. See `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`. Checkboxes below are DEV-reported progress, not reviewer acceptance. Phase 2 is not authorized.

## Outcome

Enable a user in a daily (1D) replay session to:
1. Inspect the Signal Registry containing versioned signal definitions (`volume.relative_volume` and `volume.spike`).
2. Configure `period` and `multiplier` parameters within safe bounds without touching code.
3. Calculate Relative Volume and Volume Spike deterministically on the visible replay session prefix.
4. View values, status, quality codes, explainable reasons, and availability metadata in a dedicated Signal Inspector panel without future candle leakage.
5. Resolve signals in safe Strategy Rule Evaluator DSL through an isolated binding adapter on precomputed snapshots.
6. Guarantee zero trade execution, zero order creation, zero database mutation, and zero P&L claims in this slice.

---

## Context and Problem

Sumi requires an authoritative, explainable, and causal signal foundation. Previously:
- Indicators were calculated in backend `IndicatorEngine`, but no signal registry, availability contract, or explainability schema existed.
- Strategy evaluation supported basic AST/DSL, but lacked a formal adapter to bind precomputed signal outputs with explicit quality/availability checks.
- Backtest execution evaluated close-derived signals at same-bar close (known non-causal). Phase 1 isolates signal calculation entirely from automated backtesting.
- This first vertical batch implements the core signal contract (`P1-SIG-01`) and the first end-to-end user-facing signal (`P1-SIG-02`: Volume Spike) based on audited OHLCV data.

Addressed Acceptance IDs:
- System Functional: `FR-CORE-001`, `FR-CORE-002`, `FR-CORE-003`, `FR-CORE-004`, `FR-CORE-006`, `FR-CORE-007`, `FR-CORE-011`
- Non-Functional: `NFR-DET-001`, `NFR-DQ-001`, `NFR-MAINT-001`, `NFR-UX-001`
- Domain Signals: `SIG-VOL-001` (Relative Volume), `SIG-VOL-002` (Volume Spike)
- UI & Explainability: `UI-SIG-001`, `UI-SIG-002`, `UI-SIG-003`, `UI-EXP-001`
- Verification & Test: `TEST-SIG-001`, `TEST-SIG-002`, `TEST-SIG-003`, `TEST-DSL-001`, `TEST-CAUSAL-001`
- Product Acceptance: `G-01`..`05`, `R-01`, `R-04`

---

## In Scope

- **Signal Domain (`P1-SIG-01`):**
  - Pure signal models: `SignalDefinition`, `SignalParameter`, `SignalOutputPoint`, `SignalSeriesResult`, `SignalQuality`.
  - Signal Registry: Singleton/in-memory registry with `volume.relative_volume` and `volume.spike` version `1.0.0`.
  - Shared candle/volume feature frame: Chronological validation, monotonic checks, prior-N window slicing excluding current bar.
  - Deterministic canonical params hash (canonical UTF-8 JSON, sorted keys, compact separators, SHA-256).
- **Volume Spike Capability (`P1-SIG-02`):**
  - Exact formula: `baseline[t] = mean(volume[t-period:t])`, `RVOL = volume[t] / baseline[t]`, `spike = RVOL >= multiplier`.
  - Quality states: `VALID`, `INSUFFICIENT_HISTORY`, `INVALID_VOLUME`, `ZERO_BASELINE`.
  - Causal availability: `availability_event = BAR_CLOSE` with bar index and bar timestamp.
- **Safe Strategy DSL Binding:**
  - `SignalBindingAdapter` validating dependencies before calling `RuleEvaluator`.
  - Explicit AST aliases: `volume__relative_volume` and `volume__spike`.
  - Fail-closed: missing/invalid dependencies return nullable result with reason, preventing `not` or `any` from evaluating invalid data to true.
- **API Endpoints:**
  - `GET /api/signals/registry`
  - `POST /api/signals/replay/{session_id}/calculate` (capped at session's `current_index`).
- **UI Component:**
  - `SignalInspector.tsx` mounted cleanly in `ReplayWorkspace.tsx`.
  - Parameter controls (`period`, `multiplier`), explanation inspector, quality badges, warmup display.
  - Concurrency & stale response protection.

---

## Out of Scope

- Automated backtest execution or integration with `BacktestService`.
- Trade, order, fill, position, or execution lifecycle interaction.
- P&L calculation, win rate, or profitability recommendations.
- Money Flow / Blackbox subsystem (`OHLCV_PROXY`, Active Flow, Market BB).
- `SUMI-420` universe management.
- Other Price/Volume signals (patterns, regimes, VSA, Ichimoku, divergence).
- Chart markers, visual overlays, or series modifications on Lightweight Charts.
- Signal persistence or strategy database migrations.
- External market data provider modifications.
- Optional direction or ATR filters for Volume Spike.
- Phase 2 or subsequent phases.

---

## Invariants

- **No Future Leakage:** Signal calculation consumes only bars up to `current_index` via `ReplayService.get_candles`. No post-index data is returned or processed.
- **Backend Authority:** Formulas are calculated strictly in the backend domain. The UI only displays values and reasons returned by the server.
- **Safe AST Whitelist:** `backend/app/domain/strategy/rule_evaluator.py` is protected and unmodified. Safe AST rules evaluate over precomputed snapshots via the binding adapter.
- **Database Immutability:** `backend/sumi.db` is strictly read-only. Tests and UAT execute against temporary databases.
- **Local-First:** No telemetry, no external network calls, no vendor credential leakage.
- **Baseline Preservation:** The 159 staged deletions and 40 pre-existing modifications must not be committed, unstaged, or altered outside the allowed paths.

---

## Current Architecture

- `backend/app/main.py`: FastAPI entrypoint registering routers.
- `backend/app/domain/engine/indicator_engine.py`: Registry-backed technical indicators.
- `backend/app/services/replay_service.py`: Authoritative replay session management and `get_candles` prefix slicing.
- `backend/app/domain/strategy/rule_evaluator.py`: Whitelisted AST and declarative rule evaluator.
- `frontend/src/components/replay/ReplayWorkspace.tsx`: Main replay application workspace.
- `frontend/src/components/replay/PracticeRail.tsx`: Right sidebar housing trade controls and journal.

---

## Target Design

```text
ReplayService.get_candles(session_id) [up to current_index]
                   │
                   ▼
       Shared Candle/Volume Frame
  (validation, chronological monotonic checks)
                   │
                   ▼
              SignalEngine
  (volume.relative_volume, volume.spike v1.0.0)
                   │
                   ├──────────────────────────────────┐
                   ▼                                  ▼
      Replay Calculation API                 SignalBindingAdapter
  (POST /api/signals/replay/calculate)   (quality/availability guard)
                   │                                  │
                   ▼                                  ▼
         SignalInspector (UI)                 RuleEvaluator (Safe AST)
   (values, reasons, quality, params)     (volume__spike == True, no eval)
```

---

## Exact Allowed Files

### New Files to Create:
1. `backend/app/domain/signals/__init__.py`
2. `backend/app/domain/signals/models.py`
3. `backend/app/domain/signals/registry.py`
4. `backend/app/domain/signals/volume.py`
5. `backend/app/domain/strategy/signal_binding.py`
6. `backend/app/schemas/signal_schema.py`
7. `backend/app/services/signal_service.py`
8. `backend/app/api/signals.py`
9. `backend/app/tests/test_signals.py`
10. `backend/app/tests/test_signal_binding.py`
11. `backend/app/tests/test_signals_api.py`
12. `frontend/src/api/signalsApi.ts`
13. `frontend/src/types/signals.ts`
14. `frontend/src/components/signals/SignalInspector.tsx`
15. `frontend/src/components/signals/__tests__/SignalInspector.test.tsx`
16. `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`

### Existing Files to Modify (Minimal & Focused):
1. `backend/app/main.py` — Add `signals.router` under `/api/signals`.
2. `frontend/src/components/replay/ReplayWorkspace.tsx` — Add `SignalInspector` import and mount at the top of `PracticeRail.trade` content (after `ScannerSourceContext`, before `practiceData`). Pre-edit SHA-256 verified: `6B58401E8A3D868106CABA22F53D7DAA8D1905A282BD79C2E3A6E695579B486F`.
3. `scripts/product-uat.mjs` — Add focused Volume Spike browser assertions.

---

## Protected Files (Strictly Do Not Modify)

- `backend/app/services/backtest_service.py`
- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/domain/strategy/rule_evaluator.py`
- `backend/app/models/**`
- `backend/alembic/**`
- `frontend/src/components/replay/ReplayWorkspaceController.tsx`
- `frontend/src/components/chart/**`
- `scripts/run-comprehensive-uat.ps1`
- `scripts/comprehensive-system-uat.mjs`
- `scripts/run-product-uat.ps1`
- `backend/sumi.db`

---

## Frozen Contracts (From Section P.4)

| Contract | Required Behavior |
| :--- | :--- |
| **Identity** | Registry IDs `volume.relative_volume` (numeric) and `volume.spike` (boolean), version `1.0.0`; AST aliases `volume__relative_volume` and `volume__spike`. Explicit mapping; one resolved config per ID; reject duplicate IDs. |
| **Parameters** | `period`: strict integer, default 20, range 1–252. `multiplier`: finite positive number, default 2.0, range > 0 and <= 100. Reject bool-as-number, unknown fields, null, NaN, Infinity, invalid version. Relative Volume has only `period`. |
| **Formula** | At zero-based bar `t`, baseline is arithmetic mean of volumes `[t-period : t]`. RVOL = `volume[t] / baseline[t]`; spike = `RVOL >= multiplier` (no rounding before comparison). Prior bars only; current bar excluded. No fill-forward; zero is valid; missing is not zero. |
| **Warmup & Quality** | Quality enum: `VALID`, `INSUFFICIENT_HISTORY`, `INVALID_VOLUME`, `ZERO_BASELINE`. Before `period` prior bars exist: null value and `INSUFFICIENT_HISTORY`. Negative/non-finite volume: null and `INVALID_VOLUME`. Positive-length all-zero baseline: null and `ZERO_BASELINE`. Current zero on positive baseline: RVOL 0, spike false, quality `VALID`. No NaN or Infinity in JSON. |
| **Inputs** | Timeframe `1D` only. Chronologically increasing unique daily bars from same session symbol/timeframe/adjustment. Reject duplicate or non-monotonic timestamps. |
| **Session Boundary** | Authoritative prefix read via `ReplayService.get_candles(db, session_id)`. Client cannot override symbol, range, or index. Response capped at `session.current_index`. Warmup within visible session prefix. |
| **Availability** | Signal available only at `BAR_CLOSE` with bar index and bar timestamp. No midnight-to-close extrapolation. |
| **Output** | Echo session ID, observed index, registry ID, version, resolved params, deterministic params hash. Each point contains nullable typed value, quality code, reason codes, baseline, current volume, relative volume, threshold, availability metadata. |
| **Params Hash** | Canonical UTF-8 JSON of explicit signal name, version, resolved params, sorted keys, compact separators, normalized types, no NaN; SHA-256. Defaults omitted resolve to identical hash as explicit defaults. |
| **Safe DSL** | Use existing `RuleEvaluator` on precomputed typed snapshots via `SignalBindingAdapter`. Pre-evaluation inspects all referenced signal dependencies: any invalid/unavailable dependency aborts boolean evaluation and returns nullable result with reason. `not` cannot convert unavailable to true. |
| **API** | `GET /api/signals/registry` and `POST /api/signals/replay/{session_id}/calculate`. Request has 1–2 explicit signal requests. 404 for missing session, 422 for malformed parameters. |
| **UI Concurrency** | Key: `{sessionId, visibleIndex, requestedSignals, versions, resolvedConfig}`. Clear stale values on key change; discard late responses not matching current key; clear display while new request pending. |

---

## Numerical Acceptance Oracles (From Section P.5)

| Case ID | Input / Action | Expected Result |
| :--- | :--- | :--- |
| **P1-OR-01** | Volumes `[100, 200, 300, 400, 600]`, period 3, multiplier 2 | `t=0..2` unavailable (`INSUFFICIENT_HISTORY`); `t=3` baseline 200, RVOL 2.0, spike `true` (`VALID`); `t=4` baseline 300, RVOL 2.0, spike `true` (`VALID`). |
| **P1-OR-02** | Volumes `[100, 200, 300, 399]`, period 3, multiplier 2 | Baseline 200, RVOL 1.995, spike `false` (`VALID`). UI rounding must not make it true. |
| **P1-OR-03** | `[0, 0, 0, 100]` and `[100, 100, 100, 0]`, period 3, multiplier 2 | First: baseline 0, null value, quality `ZERO_BASELINE`. Second: baseline 100, RVOL 0.0, spike `false`, quality `VALID`. |
| **P1-OR-04** | `[100, null, 300, 400]`, negative, or infinite volume | Null value, quality `INVALID_VOLUME`. Window is not shrunk, missing is not replaced with zero. |
| **P1-OR-05** | Every prefix; append/mutate future bars | Previously available values, quality, and reasons match full-series prefix; future mutations do not alter past results. |
| **P1-OR-06** | Period 20 with 20 total bars, then bar 21 | First 20 bars (`t=0..19`) unavailable; first calculable result is zero-based index 20 (`t=20`) with 20 prior bars. |
| **P1-OR-07** | Valid spike true/false and unavailable spike evaluated through AST and explicit-object DSL | Valid cases evaluate accurately. Unavailable spike yields unavailable result even under `not`, `any`, `all`, or `eq false`. Evaluator is not called on missing dependencies. |
| **P1-OR-08** | `volume.spike`, unknown alias, arbitrary call/import/attribute expression, duplicate signal ID, invalid params | Explicit rejection with descriptive error; no widened grammar, no silent overwrite. |
| **P1-OR-09** | Replay session with index 3; client requests out-of-bounds index; rewind to index 2 | Valid response has no index > 3. Out-of-bounds rejected. Rewind yields series up to 2 and discards `t=3` state. |
| **P1-OR-10** | Delayed response for index 3; change index/symbol/config; return old response last | UI discards stale response and displays only current matching context or loading state. |
| **P1-OR-11** | Registry defaults omitted vs explicit; same inputs twice | Equal resolved params, identical canonical hash and results; changes to params/version change hash. |
| **P1-OR-12** | Registry → configure → inspect warmup/true/false/error states at 1440×1000 | UI matches API exactly; zero console/runtime errors; zero future signals; no Trade/P&L output created. |

---

## Milestones & Exit Criteria

### Milestone M0: ExecPlan & Verification Setup
- [x] Create `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`.
- [x] Verify candidate files do not exist.
- [x] Reviewer adjudication and baseline integrity verified.

### Milestone M1: Signal Domain & Numerical Tests
- [x] Implement `models.py`, `registry.py`, `volume.py`, `__init__.py`.
- [x] Implement unit tests covering oracles `P1-OR-01` through `P1-OR-06` and `P1-OR-11`.
- [x] Verify zero DB/API/FastAPI imports in formula modules.

### Milestone M2: DSL Binding, API, & Integration
- [x] Implement `signal_binding.py`, `signal_schema.py`, `signal_service.py`, `api/signals.py`.
- [x] Register router in `backend/app/main.py`.
- [x] Implement tests for binding (`test_signal_binding.py`) and API (`test_signals_api.py`).
- [x] Pass oracles `P1-OR-07`, `P1-OR-08`, `P1-OR-09`, `P1-OR-11` in a temporary database.
- [x] Confirm production `sumi.db` hash is unchanged.

### Milestone M3: UI Component & Minimal Replay Integration
- [x] Implement `signalsApi.ts`, `types/signals.ts`.
- [x] Implement `SignalInspector.tsx` and unit tests in `SignalInspector.test.tsx`.
- [x] Minimally mount `SignalInspector` in `ReplayWorkspace.tsx` at the approved location.
- [x] Pass oracles `P1-OR-10` and component tests covering all states.

### Milestone M4: Full Verification & Retained Evidence
- [x] Add Volume Spike assertions to `scripts/product-uat.mjs`.
- [x] Run focused backend and frontend tests.
- [x] Run `./scripts/verify-v2.ps1`.
- [x] Run `./scripts/run-comprehensive-uat.ps1`.
- [x] Run `./scripts/run-product-uat.ps1`.
- [x] Review 1440×1000 and 1280×800 screenshots.
- [x] Verify production database hash, WAL/SHM absence, and 159 staged deletion count.
- [x] Prepare handoff report and STOP.

---

## Verification Commands

```powershell
# 1. Focused Backend Tests (Isolated Temporary DB)
$signalTestDir = Join-Path ([IO.Path]::GetTempPath()) ('sumi-p1-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $signalTestDir -ErrorAction Stop | Out-Null
$signalTestDb = (Join-Path $signalTestDir 'tests.db').Replace('\', '/')
$env:DATABASE_URL = 'sqlite:///' + $signalTestDb
Push-Location backend
try {
  & .\.venv\Scripts\python.exe -m pytest app/tests/test_signals.py app/tests/test_signal_binding.py app/tests/test_signals_api.py -v
} finally {
  Pop-Location
  Remove-Item Env:DATABASE_URL -ErrorAction SilentlyContinue
  Remove-Item -Path $signalTestDir -Recurse -Force -ErrorAction SilentlyContinue
}

# 2. Focused Frontend Tests
Push-Location frontend
try {
  npm.cmd run test -- src/components/signals/__tests__/SignalInspector.test.tsx
} finally {
  Pop-Location
}

# 3. Fast Technical Gate
.\scripts\verify-v2.ps1

# 4. Comprehensive Playwright UAT (Sequential)
.\scripts\run-comprehensive-uat.ps1

# 5. Product Acceptance UAT (Sequential)
.\scripts\run-product-uat.ps1
```

---

## Rollback & Compatibility

- **Rollback Procedure:** Delete the 15 newly created Phase 1 files; discard the minimal additions in `backend/app/main.py`, `frontend/src/components/replay/ReplayWorkspace.tsx`, and `scripts/product-uat.mjs`. The repository will return precisely to the baseline frozen in `P0_BASELINE_FREEZE.md`.
- **Database Compatibility:** No database tables or columns are added. No migrations are executed.
- **API Compatibility:** Existing endpoints are untouched; `/api/signals/*` is purely additive.
- **Strategy Lab Compatibility:** Existing Strategy Lab and Replay practice workflows function without disruption.

---

## Progress Log

- **2026-09-12 (Session 00):** Baseline frozen, recorded in `P0_BASELINE_FREEZE.md`, and accepted by reviewer.
- **2026-09-12 (Session 01 - M0):** ExecPlan created, contracts copied verbatim, candidate file paths verified clean.
- **2026-09-13 (DEV P1R-01 — Backend Contract Hardening):**
  - **Blocker C (Request/Schema Contract Drift):**
    - Explicit `params: null` is rejected with HTTP 422 in `SignalCalculationItemRequest` via `params: Dict[str, Any]` and `@field_validator("params")`. Omitted params still default to `{}` and resolve defaults.
    - Registry metadata for `multiplier` updated to JSON-Schema-style `exclusiveMinimum: 0` and `maximum: 100` (removed arbitrary `0.01` / `0.1`).
    - Removed uncontracted `COMPLETE` from `SignalQuality` enum, leaving exactly the 4 frozen qualities (`VALID`, `INSUFFICIENT_HISTORY`, `INVALID_VOLUME`, `ZERO_BASELINE`).
    - `compute_canonical_params_hash` updated to accept only finite non-bool integers/floats; rejects `bool`, `null`, strings, containers, `NaN`, and `Infinity`; preserved existing golden fixture hash.
  - **Blocker D (Derived Non-Finite Values):**
    - In `calculate_relative_volume` and `calculate_volume_spike`, ensure derived baseline and RVOL are finite before returning `VALID`.
    - If extreme finite inputs produce non-finite baseline or RVOL, returns null value with `quality=SignalQuality.INVALID_VOLUME`, `reasons=["NON_FINITE_DERIVED_VALUE"]`, and no non-finite explanation fields (`None`).
    - In `SignalOutputPoint.to_dict()`, sanitized values to never serialize `NaN` or `Infinity`.
  - **Blocker B (DSL Crosses & Previous Signal Resolution):**
    - Added optional `previous_signal_points: Optional[Dict[str, SignalOutputPoint]] = None` after existing arguments in `SignalBindingAdapter.evaluate`, preserving existing callers.
    - For registered signal aliases used directly as cross operands in `cross_up` / `cross_down` DSL rules, required previous point, validated current and previous quality/value before evaluation, and bound `previous_<alias>` to `values`.
    - Missing or invalid previous signal point returns nullable/fail-closed (`is_valid=False`, `value=None`) with explicit reason, preventing `not` or `any` from evaluating invalid data to true. Numeric cross operands require no previous point.
  - **Verification:**
    - Focused backend tests: 24/24 passed in temporary SQLite database (`test_signals.py`, `test_signal_binding.py`, `test_signals_api.py`).
    - Full backend test suite: 216/216 passed in temporary SQLite database (`backend/app/tests`).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, zero `-wal` / `-shm` sidecars before and after.
    - Scope integrity: Only allowed 5 production files and 3 test files modified. No frontend, routes, services, or execution files modified. No browser UAT run in this backend-only batch.
    - Status: `P1R-01 DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`.
- **2026-09-13 (DEV P1R-01A — Signal Snapshot Integrity):**
  - **Snapshot Validation & Output Type Integrity:**
    - In `SignalBindingAdapter.evaluate`, added `_validate_snapshot_integrity` helper to strictly validate that any referenced `VALID` snapshot matches its registry definition:
      - Bool signals must have exact `bool` value (`isinstance(v, bool)`).
      - Float signals must have finite `int`/`float` value (`not isinstance(v, bool)` and `math.isfinite(v)`).
      - `output_type` must match the registry definition (`SignalOutputType`).
      - Malformed values fail closed with `value=None`, `is_valid=False`, quality `SignalQuality.INVALID_VOLUME`, and descriptive reason.
    - Updated dotted signal rejection in AST rules from naive string checking (`"." in rule`) to AST Attribute inspection (`ast.Attribute`), permitting valid floating point literals (e.g. `2.0`) in expressions while strictly rejecting dotted signal names like `volume.spike`.
  - **Cross Operands & Exact Adjacency:**
    - For each signal alias in `cross_up` / `cross_down`, enforced:
      - Previous point must be exactly `current.bar_index - 1`.
      - Missing, future, same-bar, or non-adjacent previous points fail closed with `value=None`, `is_valid=False`, and quality `SignalQuality.INSUFFICIENT_HISTORY`.
      - Both current and previous points must have `available_at_index == bar_index` and `availability_event == "BAR_CLOSE"`. Malformed availability metadata fails closed with `quality=SignalQuality.INVALID_VOLUME`.
  - **SignalOutputPoint Serialization Integrity:**
    - In `SignalOutputPoint.to_dict()`, explicitly reject internally invalid `VALID` points (e.g. null value, NaN/Infinity, or value type mismatching `output_type`) by raising `ValueError`.
    - Preserved existing valid bool/float serialization and non-VALID null serialization unchanged.
  - **Verification:**
    - Focused backend tests: 29/29 passed in temporary SQLite database (`test_signals.py`, `test_signal_binding.py`, `test_signals_api.py`).
    - Full backend test suite: 221/221 passed in temporary SQLite database (`backend/app/tests`).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, zero `-wal` / `-shm` sidecars before and after.
    - Scope integrity: Only allowed 2 domain files (`models.py`, `signal_binding.py`), 2 test files (`test_signals.py`, `test_signal_binding.py`), and `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md` modified. No other files touched.
    - Status: `P1R-01A DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`.
- **2026-09-13 (DEV P1R-01B — Numeric and Serialization Boundary):**
  - **Shared Conversion & Validation Helper (`to_finite_float`):**
    - Implemented `to_finite_float(val: Any) -> Optional[float]` in `models.py`:
      - Safely converts `int` or `float` (excluding `bool`) to a finite Python float.
      - Catches `(OverflowError, ValueError)` and filters non-finites (`math.isnan`, `math.isinf`).
      - Accurately and safely handles arbitrarily large integers (such as `10**10000`) without raising `OverflowError`.
  - **Binding Fail-Closed Behavior (`signal_binding.py`):**
    - Updated `_validate_snapshot_integrity` to use `to_finite_float`:
      - Arbitrarily large integer float snapshots fail closed with `is_valid=False`, `value=None`, quality `SignalQuality.INVALID_VOLUME`, and reason `MALFORMED_SIGNAL_VALUE_{prefix}{alias}_EXPECTED_FLOAT` prior to evaluation.
      - Enum snapshot values validate `isinstance(point.value, str)` and not `bool`.
  - **Direct Serialization Boundary (`SignalOutputPoint.to_dict()`):**
    - Restricted `output_type` to known types: `bool`, `float`, `enum`. Unknown output types raise explicit `ValueError`.
    - Strict type enforcement on `VALID` values:
      - `bool` → exact bool;
      - `float` → finite representable number (raises explicit `ValueError` on non-convertible / non-finite / oversized ints);
      - `enum` → string (raises explicit `ValueError` on bool / numeric / non-string).
    - Avoided string formatting of oversized integers in error messages to prevent exceeding Python's integer string conversion limit.
    - Numeric explanation fields (`baseline`, `current_volume`, `relative_volume`, `threshold`) sanitize oversized integers to `None` without raising or emitting non-finites.
  - **Verification:**
    - Focused backend tests: 30/30 passed in temporary SQLite database (`test_signals.py`, `test_signal_binding.py`, `test_signals_api.py`).
    - Full backend test suite: 222/222 passed in temporary SQLite database (`backend/app/tests`).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, zero `-wal` / `-shm` sidecars before and after.
    - Scope integrity: Only 4 allowed code/test files and 1 exec-plan modified.
    - Status: `P1R-01B DONE; P1R-02 NOT STARTED; PHASE 2 NOT STARTED`.
- **2026-09-13 (DEV P1R-02 — UI Request and Response Identity):**
  - **Monotonic Request Generation & Stale Invalidation:**
    - Replaced value-based request key ref tracking with a monotonically increasing request generation counter (`requestIdRef.current`).
    - Invalidates in-flight tokens on unmount, cleanup, or invalid/no-session/unsupported context.
    - Any late response or rejected promise from an older generation changes no visible state and neither renders nor ends loading, eliminating ABA stale displays (`3 -> 4 -> 3`).
  - **Strict Response Identity & Point Validation:**
    - Immediately hides prior point, params hash, error, and displays loading indicator on every new context/request.
    - Validates server response envelope: `session_id`, `observed_current_index`, `timeframe` ('1D'), `signal_name` ('volume.spike'), `signal_version` ('1.0.0'), non-empty `params_hash`, and resolved parameters (`period`, `multiplier`).
    - Requires exactly one matching point for requested index; rejects future points (`bar_index > index`), mismatched availability index/event, and mismatched timestamp against `currentTimestamp`. Removed last-point fallback.
  - **Server-Authoritative Result Rendering & Type Contracts:**
    - Results render server-authoritative evidence: `resolved_params.period`, point `threshold`, and `params_hash` (not local React state).
    - Removed `COMPLETE` from `SignalQuality` enum and component styling.
    - Aligned `SignalParameterSchema` with backend schema (`minimum`, `exclusiveMinimum`, `maximum`).
    - Allowed enum string values in `SignalOutputPoint.value` (`boolean | number | string | null`).
  - **Safe Input Boundaries & Formatting:**
    - Period input enforces strict integer 1–252 without truncation; non-integers (e.g. `1.5`) and out-of-range values display validation error and do not call the API.
    - Multiplier accepts any finite value `> 0` and `<= 100` (including `0.001`); non-positive, NaN, Infinity, or empty input display validation error and do not call the API.
  - **ReplayWorkspace Mount:**
    - Added only `currentTimestamp={currentCandle?.timestamp}` to the existing `SignalInspector` mount.
  - **Verification:**
    - Focused component tests: 12/12 passed (`SignalInspector.test.tsx`), covering ABA sequence, late rejected promises, mismatched response identity rejections, server evidence rendering, and input boundaries.
    - Full frontend test suite: 32 files / 205 tests passed.
    - Frontend lint: 0 errors, 0 warnings.
    - Frontend production build: exit 0.
    - Fast technical gate (`.\scripts\verify-v2.ps1`): exit 0 (backend 222 passed, alembic upgrade passed, frontend lint/test/build passed).
    - Comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`): 31/31 passed (100.0%).
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit 0 (passed all checks including Volume Spike Inspector assertions at line 571 and screenshot retention).
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-13T08-08-57-639Z/p1-volume-spike-inspector-1440x1000.png`.
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` before and after; zero `-wal` / `-shm` sidecars.
    - Scope integrity: Only allowed 4 code/test files (`SignalInspector.tsx`, `SignalInspector.test.tsx`, `signals.ts`, `ReplayWorkspace.tsx`) and 1 exec-plan modified.
    - Status: `P1R-02 DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`.
- **2026-09-13 (DEV P1R-02A — Render Identity and Settled Rejection):**
  - **Render Identity & Occurrence Invalidation:**
    - Replaced reusable value-key tracking with an occurrence identity token (`useMemo(() => Symbol(currentKey ?? 'invalid'), [currentKey])`).
    - The occurrence token is tied to accepted result state (`AcceptedSignalData.occurrenceToken`) and error state (`ApiErrorData.occurrenceToken`).
    - Any change in `currentKey` (including `A → B → A`, or multiplier `2 → 2.0000005`) produces a brand new unique `occurrenceToken`.
    - Visibility of results and errors is derived during render from occurrence identity match (`acceptedData.occurrenceToken === occurrenceToken`), immediately hiding completed results and errors without synchronous `setState` in `useEffect` and keeping ESLint green.
    - Preserved monotonic request generation (`requestIdRef.current`) to immediately discard late-resolving promises and late-rejecting errors.
  - **Strict Multiplier & Parameter Matching (Zero Epsilon):**
    - Removed all epsilon / tolerance comparisons (`Math.abs(...) < 1e-6` and `> 1e-6`).
    - Enforced exact equality (`acceptedData.multiplier === parsedMultiplier` and `spikeResult.resolved_params.multiplier === capturedMultiplier`).
    - Changing multiplier `2` to `2.0000005` immediately hides the old result and starts loading; a response resolved as `2` is rejected with a controlled error; exact `2.0000005` is accepted.
  - **Settled Rejection with Unified Validation Exit Helper:**
    - Every current-generation response mismatch settles as a visible controlled error (`data-testid="signal-error"`) and no result (`signal-results` absent, `activeLoading` false)—never an endless loading spinner.
    - Unified validation exits under a single `failValidation(message: string)` helper function.
  - **Comprehensive Daily Timestamp & Availability Verification:**
    - Implemented `isTimestampMatching`: when `currentTimestamp` is supplied, both `point.timestamp` and `point.available_at_timestamp` must be present, non-empty, parseable, and match the requested daily date (`toDateKey(currentTimestamp)`).
    - Missing or empty point timestamp, empty available_at_timestamp, or mismatched dates are rejected with controlled error.
    - Preserved accepted ISO/date normalization only when both values are present and parseable.
  - **Verification:**
    - Focused component tests: 15/15 passed (`SignalInspector.test.tsx`), covering:
      1. Warmup state (INSUFFICIENT_HISTORY)
      2. Active spike true state
      3. Active spike false state
      4. Validation error on bounds
      5. Unsupported timeframe warning
      6. API error handling
      7. P1-OR-10: delayed stale response discarded
      8. ABA: old index-3 response during new index-3 request neither renders nor ends loading; only new response renders
      9. Late rejected promise cannot replace newer success or show error
      10. Completed A → pending B → pending new A: completed A is hidden and loading remains until new A resolves
      11. Multiplier `2 → 2.0000005`: old result is hidden; response resolved as 2 is rejected; exact 2.0000005 accepted
      12. Prior A error → B → new A: prior error is hidden and new A shows loading
      13. Mismatched response cases (15 distinct envelope, parameter, point, future, availability, and timestamp permutations) all await the controlled error before asserting no result
      14. Returned period/threshold/hash rendered from accepted server response (no fallback)
      15. Period 1.5 and multiplier 0 do not call API; multiplier 0.001 does
    - Full frontend test suite: 32 files / 208 tests passed.
    - Frontend lint: 0 errors, 0 warnings.
    - Frontend production build: exit 0.
    - Fast technical gate (`.\scripts\verify-v2.ps1`): exit 0 (backend 222 passed, alembic upgrade passed, frontend lint/test/build passed).
    - Comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`): 31/31 passed (100.0%).
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit 0.
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-13T12-55-12-636Z/p1-volume-spike-inspector-1440x1000.png`.
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` before and after; zero `-wal` / `-shm` sidecars.
    - Scope integrity: Only allowed 2 code/test files (`SignalInspector.tsx`, `SignalInspector.test.tsx`) and 1 exec-plan modified.
    - Status: `P1R-02A DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`.
- **2026-09-13 (DEV P1R-02B — Strict Daily Timestamp Validation):**
  - **Local Strict Daily-Date Parser (`parseStrictDailyDate`):**
    - Implemented a local parser in `SignalInspector.tsx` without mutating shared `date.ts`.
    - Validates real `YYYY-MM-DD` calendar dates via UTC (`Date.UTC`) to prevent timezone shifting of Vietnam market dates.
    - When time suffix exists, strictly validates the entire `YYYY-MM-DD[T or space]HH:mm:ss...` string including clock boundaries (hours 0–23, minutes 0–59, seconds 0–59, timezone offset hours 0–14, minutes 0–59).
    - Rejects impossible dates (e.g. `2026-02-30`), malformed suffixes (e.g. `2026-01-06Tnot-a-time`), and arbitrary text.
    - Accepts valid date-only (`2026-01-06`), ISO datetime (`2026-01-06T15:30:00Z`), valid offset ISO (`2026-01-06T15:30:00+07:00`), and space-separated backend datetime (`2026-01-06 15:30:00`).
  - **Strict `currentTimestamp` Prop Boundaries:**
    - `currentTimestamp === undefined` remains the supported "not supplied" case.
    - Defined but empty, whitespace-only (`"   "`), or malformed `currentTimestamp` immediately shows a controlled validation error (`data-testid="signal-validation-error"`), makes zero API calls, renders no result, and shows no loading spinner.
  - **Settled Response Timestamp Rejection:**
    - When a valid `currentTimestamp` is supplied, malformed `point.timestamp` or `point.available_at_timestamp` (such as `2026-01-06Tnot-a-time` or `2026-02-30`) or mismatched dates fail closed through `failValidation`: visible `signal-error`, no result, no loading.
  - **Preserved Concurrency & Exact Contracts:**
    - Fully preserved occurrence-token render identity (`useMemo(() => Symbol(currentKey ?? 'invalid'), [currentKey])`), exact multiplier equality, ABA protection, and late-promise cancellation.
  - **Verification:**
    - Focused component tests: 17/17 passed (`SignalInspector.test.tsx`), including:
      1. Rejection of `point.timestamp="2026-01-06Tnot-a-time"`
      2. Rejection of malformed `available_at_timestamp="2026-01-06Tnot-a-time"`
      3. Rejection of impossible calendar dates (`2026-02-30`) in `point.timestamp` and `available_at_timestamp`
      4. Defined whitespace (`"   "`), empty string (`""`), malformed suffix, and impossible date in `currentTimestamp` showing controlled validation error, 0 API calls, no loading, no result
      5. Full acceptance matrix for valid `2026-01-06`, `2026-01-06T15:30:00Z`, `2026-01-06T15:30:00+07:00`, and `2026-01-06 15:30:00`
      6. All 15 prior test suites preserved unchanged in meaning
    - Full frontend test suite: 32 files / 210 tests passed.
    - Frontend lint: 0 errors, 0 warnings.
    - Frontend production build: exit 0.
    - Fast technical gate (`.\scripts\verify-v2.ps1`): exit 0 (backend 222 passed, alembic upgrade passed, frontend lint/test/build passed).
    - Comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`): 31/31 passed (100.0%).
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit 0.
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-13T14-06-58-531Z/p1-volume-spike-inspector-1440x1000.png`.
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` before and after; zero `-wal` / `-shm` sidecars.
    - Scope integrity: Only allowed 2 code/test files (`SignalInspector.tsx`, `SignalInspector.test.tsx`) and 1 exec-plan modified.
    - Status: `P1R-02B DONE; P1R-03 NOT STARTED; PHASE 2 NOT STARTED`.

- [x] **P1R-03A — Incremental Scope & Drift Inventory** (2026-09-13):
  - **Comprehensive Audit Delivered:** Full audit recorded in `docs/reviews/P1R_03_SCOPE_INVENTORY.md`.
  - **Bitwise & Cryptographic Baseline Verification:**
    - Git HEAD bitwise identical: `89e04fb00210027c8b08d3fd0116b5773ac26f17` (branch `master`).
    - Staged entries: Exactly 159 deletions (`D`), zero additions, modifications, or renames (100% match).
    - Unstaged tracked modifications: Exactly 43 files relative to HEAD (40 baseline + 3 clean-at-HEAD files: `backend/app/main.py`, `frontend/src/components/common/SessionPicker.tsx`, `scripts/product-uat.mjs`).
    - Baseline manifest audit: 93/93 paths verified (80 unchanged, 13 changed post-freeze, 0 missing).
  - **Untracked Three-Snapshot Progression:**
    - P1R-03A pre-report snapshot: 80 untracked total (27 new: 15 code + 12 docs).
    - P1R-03A post-report snapshot: 81 untracked total (28 new: 15 code + 13 docs, including `P1R_03_SCOPE_INVENTORY.md`).
    - P1R-03B start state: 82 untracked total (29 new: 15 code + 14 docs, including `P1R_03B_UAT_SCOPE_CLOSURE.md`).
    - Exact 15 Phase 1 code/test files identified; zero unexpected source or binary files.
  - **Classification & Governance Boundaries:**
    - Reclassified `AGENTS.md`, `docs/research/DORAEMON_MARKET_DATA_AUDIT.md`, and `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` as `REVIEWER_GOVERNANCE` with recommendation `KEEP_SEPARATE` (never `KEEP_AS_P1` product code).
    - Added Section 5.3 Exact Phase 1 Include Manifest (15 new files + 3 integration hunks).
    - Added Section 5.4 Separate-Drift Manifest (hunk-level separation for `ReplayWorkspace.tsx` and `product-uat.mjs`; explicitly documented that separate drift is neither physically cleaned nor accepted).

- [x] **P1R-03B — Phase 1 UAT and Scope Closure** (2026-09-13):
  - **Signal Inspector UAT Hardening (`scripts/product-uat.mjs`):**
    - Verified `/api/signals/registry` contains valid `volume.spike` v1.0.0 contract (`output_type: 'bool'`, `category: 'volume'`).
    - Implemented `fetchApiSignalPoint`:
      - Strict bar identity: selects exactly one point with `bar_index === observed_current_index`.
      - Fail-closed on missing point or duplicate points for `observed_current_index`.
      - Strict prefix invariance: asserts zero future points beyond `observed_current_index` (`futurePoints.length === 0`).
      - Completely eliminated `.at(-1)` indexing.
    - Full Browser State Coverage Verified:
      - Initial state: `period=20, multiplier=2.0` verified `VALID` and UI matches API point verbatim.
      - Insufficient history state: `period=150` on ~60 bars verified `INSUFFICIENT_HISTORY`, `value: null`, UI shows `CHƯA ĐỦ DỮ LIỆU` and `N/A`.
      - Spike true state: dynamically calculated `multiplier < RVOL` verified `value: true`, UI shows `ĐỘT BIẾN`.
      - Spike false state: dynamically calculated `multiplier > RVOL` verified `value: false`, UI shows `BÌNH THƯỜNG`.
      - Controlled API error: injected via narrowly scoped Playwright route on `POST **/api/signals/replay/${sessionId}/calculate`. Verified `data-testid="signal-error"` visible, results and loading absent. Route removed in `finally`.
      - Restored valid state: restored `period=20, multiplier=2.0` verified strictly `VALID` (rejecting `COMPLETE`). UI matches backend point verbatim.
    - Timing & Test Harness Resilience:
      - Replaced fixed 400ms sleeps with response/state-based synchronization (`setParamsAndWait` using `page.waitForResponse` and `page.waitForFunction`).
      - Preserved Practice UAT console isolation by capturing and restoring `expectedPracticeConsoleErrors.length` across the controlled error window.
      - Removed undeclared `check('p1...')` calls, achieving zero unexpected reconciliation IDs.
      - Retained 1440×1000 screenshots: `p1-volume-spike-inspector-1440x1000.png` and `signal-inspector-1440x1000.png`.
  - **Full Verification Suite:**
    - UAT syntax check (`node -c scripts/product-uat.mjs`): exit code 0.
    - Focused SignalInspector tests: 17/17 passed (`SignalInspector.test.tsx`).
    - Fast technical gate (`.\scripts\verify-v2.ps1`): exit code 0 (222 backend passed, alembic upgrade passed, frontend lint clean, 210 frontend passed, production build passed).
    - Comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`): 31/31 passed (100.0%).
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit code 0 (`passed: 342`, `failed: 0`, `blockingFailed: 0`, `reconciliationPass: true`, `runtimeErrors: 0`, `dbUnchanged: true`).
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-13T16-33-22-799Z/p1-volume-spike-inspector-1440x1000.png` (verified crisp 1440×1000 layout with VALID badge, BÌNH THƯỜNG, 20-period baseline, bar close context, and canonical hash).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` before and after; zero `-wal` / `-shm` sidecars.
- [x] **P1R-03C — UAT Evidence Determinism & Final Scope Seal** (2026-09-14):
  - **Local Request Predicate (`isMatchingCalculateRequest`):**
    - Added a safe, robust local predicate in the Phase 1 block of `scripts/product-uat.mjs` that parses `request.postDataJSON()`.
    - Matches POST method, exact pathname `/api/signals/replay/${sessionId}/calculate`, exactly one `signals` item, signal name `volume.spike`, version `1.0.0`, and strict numeric equality on `params.period` and `params.multiplier`.
    - Integrated into `setParamsAndWait`: replaces ambiguous URL/path-only listeners with exact request payload matching on `page.waitForResponse()`, cleanly ignoring intermediate API calls fired while filling consecutive input fields.
  - **Narrowly Scoped Controlled Error & Evidence Retention:**
    - Controlled error route strictly scoped via `isMatchingCalculateRequest(route.request(), 20, 2.1)`.
    - Used `route.abort('aborted')` with targeted in-page `window.console.error` filtering to prevent uncaught network abort noise from leaking into `page.on('console')`.
    - Zero array truncation, splicing, or filtering: `expectedPracticeConsoleErrors`, `runtimeErrors`, `requestFailures`, and `apiOutcomes` are preserved intact.
    - Captures the exact signal request failure (`requestFailures.slice(initialFailuresCount)`) and validates exactly one matching failure with method `POST` and url containing `/calculate`.
    - Concludes `negativeTracker.endOperation('signal-controlled-error')` asserting `pass === true`.
    - Persists `{ snapshot: signalOpSnapshot, requestFailure: signalRequestFailure }` under `hardening.signalControlledError` in `results.json` for independent review.
  - **Deterministic Valid State Restoration:**
    - Restored `20` / `2.0` via `setParamsAndWait(20, 2.0, 'VALID')` with exact-response helper.
    - Asserts UI badges (`VALID`, `BÌNH THƯỜNG`), metrics, and reasons match backend API point verbatim.
  - **Inventory Recalculation & Scope Correction:**
    - Recorded P1R-03C start state: 83 untracked total / 30 post-freeze (15 code + 15 governance docs).
    - Recalculated `scripts/product-uat.mjs` diff: 10 hunks (+298 / -9 lines).
    - Hunk 1 is lines 568–846 (+279 / -0 lines: Phase 1 Volume Spike UAT suite).
    - Hunks 2–10 are lines 1170–3157 (+19 / -9 lines: selector resilience tweaks quarantined under `KEEP_SEPARATE`).
    - Corrected stale `+72` and `568–638` claims across `docs/reviews/P1R_03_SCOPE_INVENTORY.md`.
  - **Verification Suite Evidence:**
    - Syntax check (`node --check scripts/product-uat.mjs`): exit code 0.
    - Focused `SignalInspector` tests (`SignalInspector.test.tsx`): 17/17 passed.
    - Fast technical gate (`.\scripts\verify-v2.ps1`): exit code 0 (backend 222 passed, alembic upgrade passed, frontend lint clean, 210 frontend tests passed, frontend build passed).
    - Comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`): 31/31 passed (100.0%).
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit code 0 (`blockingFailed: 0`, `failed: 0`, `reconciliation.pass: true`, `runtimeErrors: 0`, `hardening.signalControlledError` passed, exact 1 signal request failure captured).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` bitwise identical before and after; zero `-wal` / `-shm` sidecars.
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-14T15-18-15-033Z/p1-volume-spike-inspector-1440x1000.png` (crisp 1440×1000 viewport, VALID badge, BÌNH THƯỜNG, bar #64 close match, hash).
    - Scope integrity: Only allowed 3 files touched (`scripts/product-uat.mjs` Phase 1 block, `docs/reviews/P1R_03_SCOPE_INVENTORY.md`, `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`).
    - Final Status: `P1R-03C DONE; PHASE 1 AWAITS REVIEWER SEAL; PHASE 2 NOT STARTED`.

- [x] **P1R-03D — Remove UAT Evidence Suppression & Final Verification** (2026-09-15):
  - **Zero Evidence Suppression:**
    - Completely deleted both `page.evaluate` blocks replacing and restoring `window.console.error`.
    - Zero console, network, listener, or evidence filtering introduced; all global listeners and collections operate untampered.
  - **Malformed HTTP-200 Contract Injection:**
    - Extended local `fetchApiSignalPoint` return value with `timeframe: data.timeframe`.
    - Replaced `route.abort` with deterministic `route.fulfill` on the exact `period=20, multiplier=2.1` request, returning HTTP 200 JSON `{ session_id: sessionId, observed_current_index: initialData.observedIndex, timeframe: initialData.timeframe, results: [] }`.
    - Drives UI response-contract validation fail-closed to `signal-error` (`Thiếu kết quả tín hiệu volume.spike trong phản hồi`) with `signal-results` and `signal-loading` absent, generating zero console errors or network abort noise.
  - **Non-Tautological Negative Operation Tracking:**
    - Configured `negativeTracker.startOperation('signal-controlled-error')` with `expectedEndpoint: /api/signals/replay/${sessionId}/calculate` and `expectedStatus: 200`. Removed `allowNoResponses`.
    - Asserted `signalOpSnapshot.pass === true`, `signalOpSnapshot.capturedResponseCount === 1`, and matching captured URL and HTTP 200 status.
    - Captured `injectedRequestBody` and `injectedResponseBody` in the route handler. Asserted route fired once and zero new request failures occurred for this exact Signal endpoint (`signalRequestFailureDelta === 0`).
    - Persisted full evidence under `hardening.signalControlledError`: `{ snapshot, injectedRequest, injectedResponse, signalRequestFailureDelta: 0 }`.
  - **Deterministic Valid State Restoration & Screenshot:**
    - Restored `20` / `2.0` via `setParamsAndWait(20, 2.0, 'VALID')` with exact-response helper.
    - Asserts UI badges (`VALID`, `BÌNH THƯỜNG`), metrics, and reasons match backend API point verbatim.
    - Retained and visually verified 1440×1000 screenshots (`p1-volume-spike-inspector-1440x1000.png` and `signal-inspector-1440x1000.png`).
  - **Inventory Recalculation & Scope Correction:**
    - Recorded P1R-03D start state: 84 untracked total / 31 post-freeze (15 code + 16 governance docs).
    - Recalculated `scripts/product-uat.mjs` diff: 10 hunks (+306 / -9 lines).
    - Hunk 1 is lines 568–854 (+287 / -0 lines: Phase 1 Volume Spike UAT suite).
    - Hunks 2–10 are lines 1170–3157 (+19 / -9 lines: selector resilience tweaks quarantined under `KEEP_SEPARATE`).
    - Updated `docs/reviews/P1R_03_SCOPE_INVENTORY.md` across Sections 1, 2.2, 4.2, 5.2, 5.3, 6.3, and 10.
  - **Bounded Verification Suite Evidence:**
    - Syntax check (`node --check scripts/product-uat.mjs`): exit code 0.
    - Focused `SignalInspector` tests (`SignalInspector.test.tsx`): 17/17 passed.
    - Product UAT (`.\scripts\run-product-uat.ps1`): exit code 0 (`blockingFailed: 0`, `failed: 0`, `reconciliation.pass: true`, `runtimeErrors: 0`, `hardening.signalControlledError.pass: true`, `capturedResponseCount: 1`, `signalRequestFailureDelta: 0`).
    - Production database `backend/sumi.db`: SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` bitwise identical before and after; zero `-wal` / `-shm` sidecars.
    - Retained and reviewed screenshot: `test-results/product-uat/2026-09-14T18-09-30-017Z/p1-volume-spike-inspector-1440x1000.png` (crisp 1440×1000 viewport, VALID badge, BÌNH THƯỜNG, bar #63 close match, hash).
    - Scope integrity: Only allowed 3 files touched (`scripts/product-uat.mjs` Phase 1 block, `docs/reviews/P1R_03_SCOPE_INVENTORY.md`, `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`).
    - Final Status: `P1R-03D DONE; PHASE 1 AWAITS REVIEWER SEAL; PHASE 2 NOT STARTED`.



