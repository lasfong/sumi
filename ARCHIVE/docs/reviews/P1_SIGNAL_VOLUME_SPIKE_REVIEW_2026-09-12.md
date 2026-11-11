# P1 Signal / Volume Spike — Independent Reviewer Verdict

Date: 2026-09-12  
Baseline: `89e04fb00210027c8b08d3fd0116b5773ac26f17` plus the working-tree snapshot in `P0_BASELINE_FREEZE.md`  
Verdict: **REWORK_REQUIRED — PHASE 1 IS NOT SEALED; PHASE 2 IS NOT AUTHORIZED**

## Bottom line

The pure prior-window Volume Spike formula and server-side replay prefix are substantially correct and salvageable. The DEV report cannot be accepted as a Phase 1 completion report because:

1. `P1-OR-10` is demonstrably false for an ABA request sequence.
2. The DSL adapter cannot supply/check previous signal values for `cross_up` / `cross_down`, despite the frozen contract.
3. Several parameter and finite-number contracts are inconsistent or bypassable.
4. The implementation session changed unrelated production/UI/test surfaces after the frozen baseline.
5. The product UAT changes include unrelated assertion changes and do not prove all states claimed by `P1-OR-12`.

Do not discard the whole implementation. Preserve it, repair it in small reviewer-gated batches, and do not start Phase 2.

## Independently reproduced evidence

### Gates that really pass on the current mixed working tree

- Focused backend tests in a fresh temporary SQLite database: `17 passed`.
- Existing Signal Inspector component tests: `7 passed`.
- Full backend gate: `209 passed`.
- Full frontend suite: `32 files / 200 tests passed`.
- Frontend lint: exit `0`.
- Frontend production build: exit `0`.
- Comprehensive browser UAT rerun: `31/31 passed`.
- Production DB before/after: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`.
- `backend/sumi.db-wal` and `backend/sumi.db-shm`: absent.
- Staged deletion count remains `159`.

These green counts establish that the current mixed tree runs. They do not establish Phase 1 acceptance because the required adversarial cases are missing and unrelated code/test changes are mixed into the result.

### Reproduced blocker A — stale ABA response is displayed

Sequence:

1. Start a request for index 3.
2. Move to index 4 and display its response.
3. Return to index 3 and start a new request.
4. Resolve the *old* index-3 request while the new index-3 request is still pending.

Observed result: the UI displays `STALE_OLD_3`. The request-key ref compares only value equality, so returning to the same key makes an obsolete request appear current. The independent adversarial Vitest fails as expected. Diagnostic source is retained at:

`scratch/review-phase1-baseline/SignalInspector.adversarial.test.tsx`

The production defect is in `SignalInspector.tsx`: key construction/request tracking near lines 28–93. A monotonically increasing generation/token is required in addition to a context key. The component must also reject a response whose `session_id`, `observed_current_index`, timeframe, signal version, resolved params/hash, or exact requested point does not match. Falling back to the last point is not acceptable.

The explanation panel also renders its local `period` / `multiplier` state instead of the server-returned threshold/resolved parameters. That does not satisfy M3's “backend values displayed verbatim” rule when the server normalizes or rejects a configuration.

### Reproduced blocker B — DSL crosses cannot resolve previous signal state

`SignalBindingAdapter.evaluate({"cross_up":["volume__relative_volume",2.0]}, ...)` raises:

`RuleEvaluationError: Unknown identifier: previous_volume__relative_volume`

This violates P.4, which explicitly requires previous dependencies for crosses to be checked before evaluation. Existing tests cover `eq`, `not`, and `any`, but not the frozen cross dependency contract.

### Reproduced blocker C — request/schema contract drift

- Request item `{"name":"volume.spike","params":null}` is accepted and silently becomes defaults. P.4 requires null rejection and P1-OR-08 prohibits silent default substitution.
- Registry metadata declares multiplier minimum `0.01`, the UI input declares `0.1`, while backend validation correctly accepts every finite value `> 0` through `100`. Example: `0.001` is accepted by calculation but contradicted by the registry/UI.
- `SignalQuality.COMPLETE` exists in backend/frontend even though the frozen quality enum contains only `VALID`, `INSUFFICIENT_HISTORY`, `INVALID_VOLUME`, and `ZERO_BASELINE`.
- `compute_canonical_params_hash` accepts a boolean parameter even though its own contract says normalized types with no bool-as-number.

### Reproduced blocker D — derived non-finite values

Finite input volumes can overflow arithmetic:

- Prior volumes `[1e308, 1e308, 1e308]` produce `baseline=inf`, while the point is marked `VALID`; serialization changes the baseline to `null` but leaves a normal-volume result.
- Prior volumes at the smallest positive float and current volume `1e308` produce `RVOL=inf`; Relative Volume remains `VALID`, and its serialized `value` is still `inf` while `relative_volume` becomes `null`.

This violates the frozen rule “No non-finite JSON numbers” and creates internally contradictory explanations.

## Baseline and scope audit

### Authorized integration changes

- `backend/app/main.py`: small router registration — appropriate.
- Phase 1 new signal/API/test/component files — expected candidates.
- `frontend/src/components/replay/ReplayWorkspace.tsx`: only a minimal import/mount was authorized.
- `scripts/product-uat.mjs`: only focused Signal assertions were authorized.

### Unauthorized post-baseline semantic changes

- `backend/app/services/practice_workflow_service.py`: added destructive practice reset behavior.
- `frontend/src/App.tsx`: sidebar state/persistence and composition change.
- `frontend/src/components/layout/Sidebar.tsx`: navigation redesign.
- `frontend/src/components/replay/PracticeRail.tsx`: tab model/event behavior rewrite.
- `frontend/src/components/replay/TradeControls.tsx`: quick SL/TP and R:R behavior.
- `frontend/src/components/replay/ReplayWorkspace.tsx`: about `309 insertions / 209 deletions` relative to the frozen baseline, far beyond a minimal Signal Inspector mount.
- `scripts/product-uat.mjs`: about `90 insertions / 9 deletions`; besides Signal checks, drawing selectors/assertions and other UAT behavior were changed.

Several protected/pre-existing backend files also have post-baseline byte/EOL drift even where the semantic diff is empty. The DEV report's claim that all protected files had zero modification is therefore not accurate as a baseline statement.

The unrelated changes may be useful work from an older Trading Lab effort; they are not automatically rejected as product ideas. They must be quarantined and reviewed as a separate batch, not counted as Phase 1 evidence. Do not delete or restore them without a reviewer/owner decision.

## Oracle verdict

| Oracle | Reviewer status | Reason |
| --- | --- | --- |
| P1-OR-01..06 | PASS | Formula fixtures and prefix invariance are correctly asserted. |
| P1-OR-07 | PARTIAL | Missing/invalid boolean dependencies fail closed; previous signal dependencies for crosses are broken. |
| P1-OR-08 | FAIL | `params:null` silently substitutes defaults; contract metadata is inconsistent. |
| P1-OR-09 | PASS | Temporary-DB API test confirms server prefix and rewind cap. |
| P1-OR-10 | FAIL | Independent ABA test displays a stale result. |
| P1-OR-11 | PARTIAL | Default/explicit hash fixture passes; hash utility still accepts bool and lacks complete normalization enforcement. |
| P1-OR-12 | NOT ESTABLISHED | Screenshot is readable and one current state matches API, but full registry/warmup/true/false/error/future/stale browser matrix is not asserted. |

The retained `1440×1000` screenshot was visually inspected and is readable, although the inspector is dense. The existing product-UAT artifact reports `342/342`; it was inspected, not accepted as independent proof, because the same DEV session changed unrelated selectors/assertions in that harness.

## Correct next sequence for a low-capability DEV model

Only one bounded repair is authorized at a time:

1. **P1R-01 — Backend contract hardening.** Fix null/schema, finite arithmetic, canonical hash/quality enum, and previous-signal cross semantics. Backend files/tests only. Return to reviewer.
2. **P1R-02 — UI concurrency and exact-response identity.** Create only after P1R-01 is reviewed. Fix generation-based cancellation/ABA, clear stale state, remove last-point fallback, show server-returned values, and add adversarial component/browser assertions. Return to reviewer.
3. **P1R-03 — Scope quarantine/integration adjudication.** Decide whether the unrelated Trading Lab changes are preserved as their own batch or restored to baseline. Rebuild product-UAT delta so Phase 1 does not weaken/change unrelated assertions. Return to reviewer.
4. Rerun focused tests, fast gate, comprehensive UAT, product UAT, screenshots, DB hash, and incremental baseline audit.
5. Only the reviewer may change the verdict to `PHASE_1_ACCEPTED`. Phase 2 remains blocked until then.

The current launcher remains intentionally short; after the addendum below, the only authorized task is `docs/dev-prompts/P1R_01A_SNAPSHOT_INTEGRITY.md`.

## P1R-01 review addendum — 2026-09-13

Verdict: **NARROW_REWORK_REQUIRED — P1R-02 REMAINS BLOCKED**

P1R-01 respected its declared scope. Independent reruns produced `24/24` focused backend tests and `216/216` full backend tests using a temporary database. The production DB hash remains `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, with no WAL/SHM files. Schema null rejection, registry range metadata, canonical hash rejection, and calculator overflow handling are accepted.

Two adversarial gaps remain:

1. `SignalBindingAdapter` accepts a same-bar or future `previous_signal_points` entry as valid. Both returned `SignalBindingResult(value=True, is_valid=True, quality=VALID)`. The new tests used current and previous helpers that both set `bar_index=5`, so they accidentally codified the missing adjacency check instead of proving bar `t-1`.
2. A point with `quality=VALID` and `value=NaN` reaches the evaluator and returns `is_valid=True`; a direct `VALID + Infinity` point serializes as `value=null` while retaining `quality=VALID`. Typed snapshot integrity is therefore not enforced at the binding/model boundary.

The next task is only `docs/dev-prompts/P1R_01A_SNAPSHOT_INTEGRITY.md`. It is limited to signal model/binding and their tests. Do not begin UI work until this addendum is reviewed closed.

## P1R-01A review addendum — 2026-09-13

Verdict: **NARROW_REWORK_REQUIRED — P1R-02 REMAINS BLOCKED**

The declared fixes for exact previous-bar adjacency, availability metadata, bool/float registry-type matching, NaN/Infinity fail-closed behavior, and AST floating literals are present and correct for the tested cases. Independent reruns in temporary SQLite databases produced `29/29` focused tests and `221/221` full backend tests. The production DB SHA-256 remained `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`; WAL/SHM were absent before and after.

Two serialization-boundary gaps remain:

1. A registered float snapshot with value `10**10000` raises `OverflowError: int too large to convert to float` in both `_validate_snapshot_integrity()` and `SignalOutputPoint.to_dict()`. The binding contract requires nullable fail-closed behavior, and direct serialization must reject malformed `VALID` state explicitly rather than leaking an incidental conversion exception.
2. `SignalOutputPoint.to_dict()` currently serializes `output_type="enum", value=True` and unknown `output_type="bogus", value="x"` as `quality="VALID"`. This contradicts its stated output-type integrity contract. Enum values must be strings and unknown output types must be rejected.

The exact next task is `docs/dev-prompts/P1R_01B_NUMERIC_SERIALIZATION_BOUNDARY.md`. It is a model/binding boundary correction only. Do not start P1R-02 until reviewer closure.

## P1R-01B review addendum — 2026-09-13

Verdict: **ACCEPTED — BACKEND REWORK CLOSED; P1R-02 AUTHORIZED**

Independent temporary-database reruns produced `30/30` focused tests and `222/222` full backend tests. Production `backend/sumi.db` remained SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, with WAL/SHM absent before and after.

Adversarial probes confirm that `10**10000` now fails closed as `INVALID_VOLUME` for both current and previous float snapshots, direct float serialization raises `ValueError`, oversized explanation fields become null, enum strings serialize, and enum/type mismatches are rejected. No remaining P1R-01B blocker was found. The existing Starlette/httpx deprecation warning is unrelated.

The next and only authorized task is `docs/dev-prompts/P1R_02_UI_RESPONSE_IDENTITY.md`. It fixes UI generation/ABA handling, exact response identity, server-returned explanation values, and frontend contract drift. P1R-03 and Phase 2 remain blocked.

## P1R-02 review addendum — 2026-09-13

Verdict: **NARROW_REWORK_REQUIRED — P1R-03 REMAINS BLOCKED**

The reported gates are credible and independently sampled: focused component tests produced `12/12`, the full frontend suite `205/205`, lint and build passed, retained comprehensive UAT reports `31/31`, and product UAT reports `342/342`. The 1440×1000 Signal Inspector screenshot was visually reviewed and is readable. The exact timestamp prop was the only observed change at the existing `ReplayWorkspace` mount.

However, two independent adversarial tests both fail:

1. After a completed A result, moving to pending B and returning to a new A request immediately renders the completed old A result instead of loading. `AcceptedSignalData.generation` is stored but not compared for rendering; field equality makes old A current again.
2. Changing multiplier from `2` to `2.0000005` starts a new request but retains the old result because both response validation and `isCurrentData` use a `1e-6` tolerance. The frozen request configuration is exact identity, not approximate numeric equivalence.

The mismatched-response test is also not probative: its assertion that results are absent can pass on the initial empty render before the resolved invalid response is processed, after which it unmounts. Production code silently returns on mismatches, leaving the current request in an endless loading state. Timestamp validation additionally treats a missing response timestamp as a match and does not verify `available_at_timestamp`.

The exact next task is `docs/dev-prompts/P1R_02A_RENDER_IDENTITY.md`. It is limited to the Signal Inspector and its component tests. P1R-03 and Phase 2 remain blocked.

## P1R-02A review addendum — 2026-09-13

Verdict: **ONE NARROW TIMESTAMP REWORK REMAINS — P1R-03 REMAINS BLOCKED**

The occurrence-token fix, exact multiplier matching, controlled mismatch settlement, and rewritten mismatch tests are present and correct for the required cases. The focused component suite independently reran at `15/15`. The retained product-UAT artifact reports `342` passed, `0` failed, `0` blocking failures, and `0` runtime errors; its 1440×1000 Signal Inspector screenshot was visually reviewed and remains readable.

One explicit P1R-02A requirement is not implemented: timestamps must be parseable. `isTimestampMatching()` delegates to `toDateKey()`, which only splits a string at `T` or a space and does not validate either the calendar date or the remaining timestamp. Independent adversarial tests reproduce both failures:

1. With requested date `2026-01-06`, response timestamps `2026-01-06Tnot-a-time` and `2026-01-06Talso-invalid` are accepted and rendered as `VALID` instead of rejected.
2. A defined whitespace-only `currentTimestamp` accepts the response into state but can never satisfy render identity, leaving the visible UI in an endless loading state.

The temporary reviewer test file was removed after reproducing `2/2` failures. The only authorized next task is `docs/dev-prompts/P1R_02B_TIMESTAMP_VALIDATION.md`. It is a local parser/test correction; P1R-03 and Phase 2 remain blocked.

## P1R-02B review addendum — 2026-09-13

Verdict: **P1 UI/RESPONSE-IDENTITY REWORK ACCEPTED — AUTHORIZE P1R-03A INVENTORY ONLY**

The local parser validates calendar dates, complete time syntax, clock bounds, and timezone-offset bounds without shifting the leading Vietnam market date. Invalid defined `currentTimestamp` values settle immediately as validation errors with zero API calls. Invalid response timestamps settle through the controlled response-error path. Occurrence identity, exact multiplier matching, ABA protection, and late-promise rejection remain intact.

Independent verification produced `17/17` focused component tests plus `10/10` additional reviewer cases covering leap-day validity, non-leap rejection, invalid hour/minute/second/offsets, fractional seconds, space-separated datetimes, and preservation of the leading market date across offsets. Temporary reviewer tests were removed afterward.

The retained product-UAT result reports `342` passed, `0` failed, `0` blocking failures, and `0` runtime errors. The 1440×1000 screenshot is readable. Production DB SHA-256 remains `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`; WAL/SHM are absent.

P1R-02B is accepted. Phase 1 is not sealed because the frozen baseline comparison still shows post-freeze drift on pre-existing files and the product-UAT delta requires scope adjudication. The only authorized next task is the no-code inventory `docs/dev-prompts/P1R_03A_SCOPE_INVENTORY.md`; Phase 2 remains blocked.

## P1R-03A review addendum — 2026-09-13

Verdict: **INVENTORY SUBSTANTIALLY VALID; P1R-03B CLOSURE REQUIRED**

Independent checks confirm the staged set still matches the frozen 159 deletions, all 93 baseline-manifest paths exist, 13 baseline paths changed hash, and exactly 15 new source/test files match the approved Phase 1 candidate list. The router registration is minimal. The Signal Inspector import/mount/currentTimestamp is separable from the other `ReplayWorkspace.tsx` drift. The product-UAT Signal Inspector block is likewise separable from nine unrelated harness hunks. Production DB integrity remains unchanged.

Three corrections are required before Phase 1 can be sealed:

1. The report measured `80` untracked / `27` new before writing itself. Its post-report state is `81` / `28`, partitioned into 15 Phase 1 code/test files and 13 documents. Creation of the authorized P1R-03B prompt then makes the next start state `82` / `29` (15 code + 14 docs). This is documentation timing, not source drift, but the evidence must distinguish the snapshots.
2. `AGENTS.md`, `DORAEMON_MARKET_DATA_AUDIT.md`, and `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` are retained reviewer governance and must be `KEEP_SEPARATE`, not bundled into the Phase 1 product patch.
3. The current Phase 1 browser block still accepts `VALID || COMPLETE`, although `COMPLETE` was deliberately removed from the frozen quality enum. It also uses `.at(-1)` instead of exact bar identity and does not establish the full registry/warmup/true/false/error browser matrix required by `P1-OR-12`; fixed sleeps make its state transitions weaker than the component tests.

No unrelated drift should be destroyed. The exact next task is `docs/dev-prompts/P1R_03B_UAT_SCOPE_CLOSURE.md`, limited to the Phase 1 UAT block plus inventory/ExecPlan evidence. Phase 2 remains blocked.

## P1R-03B review addendum — 2026-09-14

Verdict: **NARROW HARNESS/EVIDENCE REWORK REQUIRED — PHASE 1 NOT SEALED**

The product implementation remains credible. Independent inspection confirmed exact current-bar selection, rejection of future points, the required warmup/true/false/restored states, and a readable 1440×1000 screenshot. The focused Signal Inspector suite independently passed `17/17`; UAT script syntax is valid. The retained product-UAT artifact reports `342` passed, zero failed/blocking assertions, zero runtime errors, reconciliation pass, and an unchanged production database.

Three closure defects remain in the UAT evidence:

1. `setParamsAndWait()` matches only URL and method. Filling period and multiplier separately can issue an intermediate request, so the wait is not tied to the exact requested parameters as required.
2. The controlled failure calls `negativeTracker.endOperation()` but discards its result, then truncates `expectedPracticeConsoleErrors`. The artifact retains a Signal API `net::ERR_FAILED` in `requestFailures`, but contains no durable operation snapshot proving that this precise failure was the intended one. Evidence collections must not be erased to obtain a clean summary.
3. The scope inventory calls the Phase 1 `product-uat.mjs` hunk `+72` at lines `568–638`; the inspected diff is currently a single `+225` hunk beginning at line 568, while the entire file is `+244/-9`. The document therefore is not an exact include manifest yet.

No product-code rework is authorized. The only next task is `docs/dev-prompts/P1R_03C_UAT_EVIDENCE_DETERMINISM.md`. Phase 2 remains blocked until reviewer seal.

## P1R-03C review addendum — 2026-09-14

Verdict: **EVIDENCE SUPPRESSION REMAINS — PHASE 1 NOT SEALED**

The exact request predicate is correct and removes the intermediate-request race. The inventory now agrees with the inspected diff (`product-uat.mjs` total `+298/-9`, Phase 1 hunk `+279` beginning at line 568). Independent syntax and focused-test reruns passed (`17/17`), the new screenshot is readable, and the production database remains bitwise unchanged.

The controlled-error evidence is not acceptable yet:

1. The Phase 1 block replaces `window.console.error` and suppresses messages matching `Network Error` or `API Error`. This hides events before the global evidence listener can classify them and violates the no-filtering intent of P1R-03C.
2. The persisted tracker snapshot is tautological: it has no expected endpoint or pattern, `allowNoResponses=true`, zero captured responses, and still reports `pass=true`. It proves no external observation. The artifact separately records `net::ERR_ABORTED`, but cannot associate the request payload because `requestFailures` stores only URL/method/error.

The reviewer is narrowing the browser proof to a controlled malformed HTTP-200 response. This exercises the UI response-contract fail-closed path without console suppression; Axios/network rejection remains covered by the accepted component tests. Only `docs/dev-prompts/P1R_03D_REMOVE_EVIDENCE_SUPPRESSION.md` is authorized next. Full technical and comprehensive gates need not be repeated because product code did not change; product UAT must be rerun.

## P1R-03D final review — 2026-09-15

Verdict: **PHASE_1_ACCEPTED — P1-SIG-01 + P1-SIG-02 SEALED**

Independent inspection confirms that all console interception and evidence suppression were removed. The exact `period=20`, `multiplier=2.1` UI request receives a deterministic malformed HTTP-200 contract, and the retained artifact proves one observed response at the exact endpoint/status with the injected request and response bodies. The operation is non-tautological (`allowNoResponses=false`, `capturedResponseCount=1`, `pass=true`) and the scoped Signal request-failure delta is zero.

The focused Signal Inspector suite independently passed `17/17`; the retained product-UAT artifact reports `342` passed, zero failed/blocking assertions, reconciliation pass, and zero runtime errors. The 1440×1000 screenshot was reviewed and is readable. Production `backend/sumi.db` remains SHA-256 `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`, with WAL/SHM absent. The P1R-03D inventory agrees with the inspected `scripts/product-uat.mjs` diff (`+306/-9` total; Phase 1 hunk `+287` beginning at line 568).

All Phase 1 acceptance oracles P1-OR-01 through P1-OR-12 are accepted. Phase 1 validates signal calculation, explanations, availability, isolated DSL binding, replay API identity, and UI inspection only. It does not validate legacy backtest/P&L correctness. Phase 2 remains the gate before any backtest result is trusted.
