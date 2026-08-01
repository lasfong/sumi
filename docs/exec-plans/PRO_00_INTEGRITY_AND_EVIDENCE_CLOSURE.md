# PRO-00 — Integrity and Evidence Closure

## Outcome

Scanner-created replay has an explicit, honest practice intent. Blind practice starts in the historical lookback without exposing the selected future signal through session APIs, browser state, labels, panels, markers, accessible names, cache, or WebSocket state. Signal review starts at the selected signal candle and is visibly identified as review. Product UAT is governed by a checked-in, fail-closed assertion manifest and retains reconciliation, runtime, database, and viewport evidence for successful and failed runs.

This batch stops at the independent Reviewer gate. It does not start PRO-01 and does not make a release, Professional-complete, product-complete, or “TradingView-like” claim.

## Context and problem

The Post-V3 master plan reopens V3 `R-01`: `frontend/src/components/replay/ReplayWorkspaceController.tsx` parses the persisted raw scanner `source_payload` immediately, and `frontend/src/components/replay/ReplayWorkspace.tsx` renders its future timestamp, strategy, signal type, regime, price, badge, and marker before replay reaches the selected signal. The Scanner creation endpoint also returns the raw ORM session without a response schema, so the same future fields are present in the initial API response.

The product-UAT harness has a second integrity defect. `scripts/product-uat.mjs` reads an ignored historical `results.json`; the pre-existing worktree change adds `.catch(() => null)`. The referenced file is absent in this checkout, so an ignored artifact is both the nominal baseline authority and unavailable. A retained local result at `test-results/product-uat/2026-07-25T03-25-35-256Z/results.json` contains 265 unique, passing checks and matches the ordinary current harness/check count described by the repository-controlled V3 evidence documents. The sustained canonical 277-check bundle named in the V3 evidence index is absent locally; its five closure checks include long-duration evidence that the ordinary deterministic product gate does not execute. PRO-00 will therefore use the reproducible 265-check ordinary product-gate result plus the current harness source as the accepted regression baseline, without inventing the unavailable sustained bundle.

Authority and required context:

- `docs/dev-prompts/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE_PROMPT.md`
- `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
- `docs/PRODUCT_V3_PLAN_2026-07-15.md`
- `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
- `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
- `docs/DEVELOPMENT_OPERATING_MODEL.md`
- `docs/PROJECT_REVIEW_REPORT_2026-07-15.md`
- `docs/V3_ACCEPTANCE_MATRIX.md`
- `docs/V3_EVIDENCE_INDEX.md`
- `docs/V3_RELEASE_CANDIDATE_NOTES.md`
- the three mandatory `docs/tester/` reports, treated as research only

Primary acceptance is V3 `G-01` through `G-05`, reopened `R-01`, `PRO-G-01` through `PRO-G-10`, and `PRO-INT-01` through `PRO-INT-10`. All other accepted V3 replay, indicator, drawing, and trading assertions remain blocking regressions.

## In scope

- Scanner signal → Replay creation request and response.
- Explicit `replay_intent: blind_practice | signal_review`, with omitted request intent defaulting to blind practice.
- Persisted local scanner audit state retaining the selected intent without a database migration.
- A typed, versioned, server-authoritative `source_context` compatibility view.
- Sanitized create, list, get/resume, advance, rewind, and WebSocket session state.
- Exact reveal index derived from the actual ordered session candle sequence.
- Blind/review controls and honest labels in Scanner and Replay.
- Removal of raw `source_payload` as frontend scanner display authority.
- Legacy scanner-session and malformed-payload fail-closed behavior.
- Checked-in product-UAT baseline manifest, strict validation, negative fixtures, result reconciliation metadata, and failed-run retention.
- Focused backend, frontend, harness, and real-browser tests/evidence.
- ExecPlan, retained artifacts, self-review, and Reviewer handoff.

## Out of scope

- PRO-01 metrics, Backtest, Analytics, Strategy Lab ranking, or scanner ranking changes.
- Dashboard, general session-picker, localization, catalog/import/sync, indicator expansion, risk tools, journal expansion, or strategy UX.
- Broad Replay/Scanner/persistence/WebSocket rewrites.
- New dependencies, schema migration, destructive data migration, or rewriting existing session history.
- Commit, stage, tag, push, merge, package, publish, branch, or worktree operations.
- Editing V2 release/tag evidence or changing acceptance criteria.

## Invariants

- Future candles and all future-derived data remain absent through `current_index`.
- Backend authorizes signal visibility; frontend hiding is not an integrity boundary.
- Backend `IndicatorEngine` remains authoritative.
- Full scanner audit data remains local; no telemetry or user trading/strategy/journal transmission is introduced.
- `backend/sumi.db` is never used by automated tests/UAT and must retain its exact SHA-256.
- Existing sessions remain readable through a compatibility view; no destructive migration is permitted.
- Raw scanner `source_payload` is never returned as frontend display authority.
- No dependency is added.
- No accepted UAT ID is removed, renamed, weakened, duplicated, or downgraded.
- Pre-existing dirty and untracked files remain user-owned and preserved.

## Current architecture

### Provenance and dirty-state preservation

Recorded before implementation on 2026-07-31:

- branch/status: `master...origin/master`
- `HEAD`: `e4e67c559f1cb181c5b966a155f19b4553c87fa6`
- `origin/master`: `e4e67c559f1cb181c5b966a155f19b4553c87fa6`
- `v2.0.0-rc2`: `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`
- production DB SHA-256: `4166D749119B0EBB4B9ADF418EA18442FF6E0C14AE762147CD3D0FBE20F76459`
- initial `git diff --check`: pass

Pre-existing tracked modifications:

- `docs/INDEX.md` — Post-V3 governance/index changes; preserve without rewriting as PRO-00 implementation.
- `scripts/product-uat.mjs` — one-line change makes the ignored returned-baseline read fail open with `.catch(() => null)`. PRO-00 will replace this dependency with the checked-in manifest; the final disposition will be recorded.

Pre-existing untracked inputs:

- `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (33,390 bytes)
- `docs/dev-prompts/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE_PROMPT.md` (16,672 bytes)
- nine files under `docs/tester/`, inventoried before editing and retained as research inputs
- `scripts/run-product-uat.ps1` (3,235 bytes), inspected as a pre-existing Windows runner; it provisions a temporary database but will not be silently promoted or rewritten unless a narrow PRO-00 runner need is proved

### Backend paths

- `backend/app/api/scanner.py` owns untyped Scanner replay creation request/route.
- `backend/app/services/scanner_service.py` creates the lookback/forward window and persists future signal fields in `ReplaySession.source_payload`; it always starts at index 0.
- `backend/app/schemas/replay_schema.py` exposes `source_payload` directly in `ReplaySessionResponse`.
- `backend/app/services/replay_service.py` owns create/list/get/advance/rewind and candle slicing but has no source-context translation.
- `backend/app/api/replay.py` returns ORM sessions through create/list/get/next/previous response models.
- `backend/app/api/ws_replay.py` broadcasts only the new candle; frontend later invalidates the session query. It has no explicit sanitized source context.
- `backend/app/models/replay_session.py` already has local `source_type`/`source_payload` fields, so compatibility can be implemented without schema mutation.

### Frontend paths

- `frontend/src/api/scannerApi.ts` has no replay-intent request type.
- `frontend/src/types/replay.ts` exposes raw `source_payload`.
- `frontend/src/pages/ScannerPage.tsx` has a single ambiguous Replay action.
- `frontend/src/components/replay/ReplayWorkspaceController.tsx` parses raw persisted JSON, builds a signal marker, and exposes signal data immediately.
- `frontend/src/components/replay/ReplayWorkspace.tsx` renders the Scanner badge and signal panel immediately, including accessible/hidden DOM content.
- React Query keys for create/get/navigation are refreshed from server state, so the sanitized `source_context` can remain the sole display authority across rewind/reload/resume.

### UAT paths and accepted baseline inventory

- `scripts/product-uat.mjs` contains the complete deterministic browser workflow and emits `results.json` or `partial-results.json`.
- `scripts/run-product-uat.sh` and the inspected pre-existing `scripts/run-product-uat.ps1` provision temporary SQLite databases.
- `scripts/verify-product.sh` composes the fast technical gate and standalone product UAT.
- The ignored historical 272/277 bundles named by old evidence documents are unavailable in this checkout.
- The available retained ordinary run `test-results/product-uat/2026-07-25T03-25-35-256Z/results.json` has 265 checks, 265 unique IDs, 265 pass, zero failed, and zero blocking failures. Its IDs correspond to the current ordinary harness, including the accepted V3 Batch 1–5 regression surface and `runtime.no-errors`.
- The checked-in manifest will contain these 265 IDs exactly once and map every entry to at least one V3 acceptance ID. Additive PRO-00 IDs will be appended and mapped to `R-01` and applicable `PRO-INT` IDs.

## Target design

### Scanner request and persistence

`ScannerReplaySessionRequest` gains a typed enum `replay_intent` with values `blind_practice` and `signal_review`; omission defaults to `blind_practice` and unknown values fail Pydantic validation. Scanner persists the intent inside its existing local JSON audit payload. No column or migration is added.

The service loads the actual ordered candle window and requires the signal timestamp to match a candle in that sequence. It records the derived `reveal_at_index`. Blind practice starts at index 0. Signal review starts at that derived index. If the signal cannot be represented safely in the actual sequence, creation fails instead of estimating.

### Sanitized source-context compatibility view

`ReplaySessionResponse` retains `source_type` for compatibility but emits `source_payload: null` for scanner sessions. It adds:

```text
source_context: {
  schema_version: 1
  source_type: string | null
  replay_intent: "blind_practice" | "signal_review" | null
  reveal_at_index: integer | null
  revealed: boolean
  signal: null | {
    timestamp: canonical API datetime
    type: string
    strategy: string
    price: number
    regime: string | null
  }
}
```

`ReplayService` owns translation from ORM state to this response. A typed stored scanner-payload schema validates JSON before translation. Existing scanner sessions without intent default to blind practice. Missing/malformed/incomplete scanner payloads fail closed with no signal and no raw payload exposure. Non-scanner sessions retain their current `source_type`/`source_payload` compatibility behavior and receive a null/unrevealed source context.

Every session-returning route calls this translation for create, list, get/resume, next, and previous. Scanner creation returns a typed response containing the same sanitized session. WebSocket candle messages include the sanitized `source_context` produced after the authoritative index mutation; no raw audit payload is broadcast.

### Frontend state and UI

Frontend Replay types model only the typed `source_context`; scanner raw payload parsing is deleted. Scanner results expose two actions:

- `Start blind practice` — default/recommended and sends `blind_practice`.
- `Review signal` — explicit and sends `signal_review`.

Replay always shows the honest workspace intent (`Blind practice` or `Signal review`). The signal badge, timestamp, type, strategy, regime, price, and marker are derived only from `source_context.signal` when `revealed` is true. Rewinding removes them because the next server response is unrevealed. React Query and WebSocket invalidation/updates replace, rather than merge, source context so repeated navigation cannot duplicate markers or retain stale badges.

### UAT manifest and evidence

`scripts/fixtures/product-uat-v3-baseline.json` is repository-controlled with schema version, baseline name, contract revision, and assertion entries. A dependency-free validator module:

- rejects missing/malformed manifests before browser success can be declared;
- rejects empty assertions, duplicate IDs, missing required fields/acceptance mappings, removed accepted IDs, unexpected actual IDs, duplicate actual IDs, and blocking mismatches/downgrades;
- computes SHA-256 of the exact manifest bytes;
- returns baseline/actual counts and missing/unexpected/duplicate/blocking-mismatch IDs;
- supports deterministic negative self-tests independent of the same unchecked parsing path.

The harness validates the manifest before launching/evaluating the workflow, uses manifest `blocking` values instead of prefix classification, adds only new PRO-00 assertion IDs, and retains:

- manifest path/schema/version/hash;
- baseline and actual counts plus reconciliation arrays;
- assertion-to-acceptance mapping;
- runtime, console/page, provider, request, and failed API outcomes;
- temporary database identity and production before/after hashes supplied by the runner;
- requested sustained duration and actual samples;
- screenshot file, viewport, and dimensions metadata.

Both caught failures and completed failing assertion runs write machine-readable results. The runner records the temporary DB path/identity and production DB hashes through environment metadata without reading or mutating production data from the app process.

## Milestones

1. **Baseline and design**
   - Mandatory reading, provenance, dirty/untracked inventory, DB hash, response-path audit, retained-result audit, target contracts, and acceptance mapping are recorded here.
   - Exit: no unresolved discoverable contract decision remains.
2. **Backend integrity**
   - Typed intent/default, actual-sequence reveal boundary, sanitized compatibility view, complete response-path coverage, legacy/malformed behavior, and WebSocket source context are implemented.
   - Exit: focused backend tests inspect complete serialized payloads before/at/after/rewind/reload and prove no raw future field.
3. **Frontend behavior**
   - Scanner has honest default/review actions; Replay consumes only `source_context`; labels/marker/panel obey reveal, rewind, reload, and deduplication.
   - Exit: focused frontend tests pass and raw scanner payload parsing/display paths are absent.
4. **Evidence authority**
   - Checked-in 265-ID accepted baseline plus additive PRO-00 assertions is strictly validated; negative fixtures fail closed; result metadata and failure retention are complete.
   - Exit: harness self-tests and an intentional failing run retain evidence.
5. **Product gate and handoff**
   - Execute as a separate verification phase after the implementation has been frozen: run the exact canonical standalone/full gates, review the retained 1440×1000 and 1280×800 screenshots, confirm the DB hash, and complete diff/evidence self-review.
   - Exit: only after that separate verification phase passes does DEV stop at the Reviewer gate. A development-complete state is not a Reviewer-gate claim.

### Execution sequencing adjustment — 2026-07-31

At the user's direction, implementation is now frozen before any further long-running product gate. Milestones 1–4 are implementation-complete. Existing focused/full technical results and successful browser evidence remain retained, but the final rerun of `scripts/run-product-uat.sh` and `scripts/verify-product.sh` is deferred to the dedicated verification phase. No PRO-01 work may begin while PRO-00 verification is pending.

## Acceptance mapping

| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| G-01 / PRO-G-01 | Complete code, typed contracts, strict harness | Backend/frontend/lint/build, `verify-v2`, standalone UAT, `verify-product` |
| G-02 / PRO-G-02 | Temporary-DB runners and recorded DB identity/hash | Exact production SHA-256 before/after plus UAT metadata |
| G-03 / PRO-G-03 | Success/failure result writer and screenshot metadata | Deliberate failure artifact plus green artifacts at both viewports |
| G-04 / PRO-G-05 | P0 future-signal leak closed end to end | Payload/DOM/marker/accessibility assertions before/at/after boundary |
| G-05 / PRO-G-08 | No network/provider/dependency expansion | Existing loopback-only UAT and request-origin evidence |
| PRO-G-04 / PRO-G-06 | Checked-in immutable manifest and strict reconciliation | Missing/malformed/empty/duplicate/removal/rename/downgrade/additive tests |
| PRO-G-07 | Dirty-state inventory and bounded diff | Final status/diff comparison and untracked inventory |
| PRO-G-09 | This living ExecPlan | Progress, decisions, deviations, verification, rollback, completion logs |
| PRO-G-10 | Independent review is required and not self-declared | DEV stops with Reviewer checklist and retained evidence |
| R-01 / PRO-INT-01 / PRO-INT-02 | Server-side candle boundary and source-context authorization | Complete serialized response, WebSocket, chart, DOM, marker, cache assertions |
| PRO-INT-03 | Typed request enum/default and persisted intent | Request default/invalid tests plus both Scanner actions |
| PRO-INT-04 | `signal: null` before `reveal_at_index` on every path | Create/get/resume/advance/rewind/WebSocket complete-payload tests |
| PRO-INT-05 | Exact actual-sequence reveal boundary and one derived marker | At/after boundary, repeated navigation, no-duplicate tests/UAT |
| PRO-INT-06 | Review starts at signal index and has honest label | Backend initial-index test and browser review action/label/payload |
| PRO-INT-07 | Scanner raw payload is persistence-only and response-sanitized | Source search, serialized payload tests, frontend raw-field absence |
| PRO-INT-08 | Missing intent defaults to blind compatibility view | Legacy fixture/test without schema migration |
| PRO-INT-09 | Server recomputes on get/navigation; client replaces context | Reload/rewind/resume/repeated-navigation tests and UAT |
| PRO-INT-10 | Deterministic Scanner-driven real UI workflow | Network/visible DOM/screenshots at before/at/after/review boundaries |

All accepted V3 `R-02` through `R-05`, `I-01` through `I-13`, `D-01` through `D-11`, and `T-01` through `T-05` remain represented by unchanged baseline IDs in the manifest.

## Verification commands

Focused commands will be refined only if repository discovery proves a narrower exact path:

```powershell
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
backend\.venv\Scripts\python.exe -m pytest backend/app/tests/test_scanner.py backend/app/tests/test_replay_no_future_leak.py backend/app/tests/test_ws_replay.py -q
Set-Location frontend; npm test -- --run src/pages/__tests__/ScannerPage.test.tsx src/components/replay/__tests__/ScannerSourceContext.test.tsx
node --test scripts/product-uat-manifest.test.mjs
```

Required full gates:

```bash
cd backend && pytest
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
```

On Windows, the inspected pre-existing runner may be used as:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1
```

Final checks:

```powershell
git status --short --branch
git diff --check
git diff --stat
git diff
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

## Rollback and compatibility

No migration is planned. Existing database rows stay byte-for-byte untouched. The compatibility translator interprets existing scanner rows without intent as blind practice and validates their persisted JSON before exposing a sanitized view. Non-scanner response behavior remains compatible. The frontend change is removable by reverting only the bounded PRO-00 files, but rollback would reopen the P0 leak and must not be released.

If rollback is required, it must be an explicit target-reviewed file operation; no reset, restore, clean, database rewrite, or broad deletion is authorized.

## Risks and mitigations

- **Signal timestamp not present in window:** require exact actual candle match and reject creation; never estimate.
- **Malformed legacy source JSON:** typed parse fails closed with null signal and no raw scanner payload.
- **One response path leaks:** central response construction plus route/WebSocket full-payload tests and source search.
- **Frontend stale cache after rewind:** replace source context from authoritative navigation/get responses and test rewind/reload/repeat.
- **Manifest reconstructed from unsupported history:** use the only available actual 265-check retained result, reconcile it to current harness source and controlled V3 evidence, and record absent sustained bundles rather than inventing 272/277 content.
- **Manifest validates itself through the same bug:** dependency-free validator tests write independent malformed fixtures and call exported validation/reconciliation boundaries.
- **Failed browser run loses evidence:** initialize metadata/result state before workflow and always write a result or partial result containing reconciliation and diagnostics.
- **Production DB mutation:** UAT uses a temporary DB; before/after SHA-256 is mandatory and a mismatch stops the batch.
- **Pre-existing untracked Windows runner overlap:** inspect and use as-is where possible; any necessary edit must be narrow and recorded as a deviation.
- **Scope pressure into PRO-01:** stop and report; no quantitative metric work is authorized.

Stop conditions are those in the standalone prompt: destructive migration/data loss, inability to establish the accepted baseline, material non-Scanner/Replay contract change, unrelated WebSocket rewrite, dependency need, conflicting acceptance, production DB change, overwrite/delete of pre-existing user files, or a P0/P1 fix requiring PRO-01+.

## Progress log

- 2026-07-31: Read the standalone authority, `AGENTS.md`, `PLANS.md`, all mandatory canonical/V3/evidence documents, and the three required tester research reports completely.
- 2026-07-31: Recorded branch/HEAD/origin/tag, exact initial DB SHA-256, initial diff check, the pre-existing UAT fail-open diff, tracked dirty files, and every untracked file with size/timestamp.
- 2026-07-31: Audited Scanner creation, Replay create/list/get/next/previous, candle/indicator, WebSocket, frontend controller/cache/marker/UI, and UAT runner/result paths.
- 2026-07-31: Confirmed the prompt’s referenced historical baseline and sealed bundle are absent locally. Reproduced the available 2026-07-25 ordinary result as 265/265 with 265 unique IDs and reconciled that count with the current ordinary harness and repository-controlled V3 evidence.
- 2026-07-31: Created this ExecPlan before product or test code. Milestone 1 is complete; Milestones 2–5 remain.
- 2026-07-31: Implemented the no-migration backend compatibility view, exact actual-candle reveal lookup, intent persistence/default/validation, sanitized create/list/get/next/previous/Scanner responses, review initial index, and WebSocket source context. Focused backend suite passed 16 tests.
- 2026-07-31: Removed frontend raw scanner-payload parsing and added explicit Scanner blind/review actions, typed source context, honest workspace status, reveal-only signal panel/marker, WebSocket cache replacement, and rewind/reload behavior. Focused frontend suite passed 6 tests and the production build passed.
- 2026-07-31: Reconciled and checked in the 265 reproduced accepted ordinary-gate IDs plus 10 additive blocking PRO-00 IDs. Added strict manifest loading, accepted-ID hash/count seal, actual-result reconciliation, result writer, and five independent negative/retention tests; all five pass.
- 2026-07-31: Confirmed deliberate unavailable-port UAT failure retention at `C:\tmp\sumi-pro00-launch-diagnostic\2026-07-31T10-24-27-613Z\partial-results.json` (manifest metadata, 275 missing IDs, error, and unchanged production DB recorded).
- 2026-07-31: Completed a green real-browser temporary-database UAT at `test-results/product-uat/2026-07-31T10-26-07-044Z/results.json`: 275/275, zero failed/blocking, exact manifest reconciliation, zero missing/unexpected/duplicate/blocking-mismatch IDs, and unchanged production DB. Visually reviewed `pro00-blind-boundary-1440x1000.png` and `pro00-signal-review-1280x800.png`; both show the expected honest intent label and a single authorized signal context/marker. Milestones 2–4 are complete; Milestone 5 full gates/self-review remain.
- 2026-07-31: User requested a code-first/test-later sequence after a long-running runner invocation. Confirmed no process remains listening on the UAT ports `18000` or `15173`. Froze PRO-00 implementation, deferred further long-running canonical gates to the separate verification phase, and did not start PRO-01.
- 2026-07-31: Committed and pushed the bounded 27-file PRO-00 implementation/evidence set to `origin/master` as `55ec5f9` (`fix(replay): close PRO-00 integrity and evidence gaps`). Pre-existing planning/research documents remained outside that implementation commit.
- 2026-08-01: Added the tracked cross-machine handoff and standalone verification-continuation authority. PRO-00 remains implementation-complete but verification/Reviewer-pending; PRO-01 remains unauthorized.

## Decision log

- **Use a compatibility view, not migration.** Existing `source_payload` can retain local audit data and intent; response sanitization and legacy defaulting require no database rewrite.
- **Centralize response authorization in `ReplayService`.** Route-local or frontend-only hiding would miss list/navigation/WebSocket paths and violate architecture boundaries.
- **Suppress raw scanner payload on every frontend response, including after reveal.** The sanitized view is sufficient and prevents it from becoming display authority again; non-scanner payload compatibility remains.
- **Use an exact signal-candle match.** The prompt forbids a frontend estimate; failure is safer than nearest-date guessing.
- **Use the reproducible 265-check ordinary gate as checked-in baseline.** The ignored sustained 272/277 artifacts are absent. The actual retained 265 result and current harness establish a trustworthy ordinary-gate identity; sustained closure counts remain historical evidence and are not fabricated.
- **Treat all baseline entries as blocking.** They are the accepted V3 regression surface. PRO-00 assertions are additive and blocking.
- **Replace the Scanner creation response’s top-level `signal_timestamp`.** Initial implementation sanitized only the nested session; complete-payload review found the legacy top-level timestamp was itself a blind-practice leak, so it was removed from the typed backend/frontend response.
- **WebSocket messages carry source context beside candle data.** The browser updates the cached session context by replacement and falls back to invalidation only for older messages, preserving compatibility without stale revealed state.

## Deviations log

- The worktree contains more pre-existing documentation changes than the prompt-creation snapshot; they were inventoried and preserved.
- The pre-existing untracked Windows runner required two narrow PRO-00 metadata environment variables so the harness can retain temporary database identity and production database path/hash. Its process-start design left Vite child processes behind on two failed diagnostic runs; only the exact processes started by those runs were identified and stopped. No unrelated runner behavior was promoted or rewritten.
- Git Bash/native-Windows process and path semantics made the canonical shell wrapper unreliable for cleanup. A narrow Windows-only delegation to the equivalent PowerShell runner was added, but its final canonical wrapper/full-gate rerun is intentionally deferred under the revised code-first/test-later sequence.

## Completion evidence

Implementation is complete and frozen; final Reviewer-gate verification is pending by explicit sequencing decision. This section will be finalized during the separate verification phase with:

- implementation commit: `55ec5f9` on `origin/master`;
- implemented surface: sanitized backend Scanner/Replay contracts, frontend reveal authorization, strict checked-in UAT manifest/harness, focused tests, and platform runners;
- retained green browser result: `test-results/product-uat/2026-07-31T10-26-07-044Z/results.json`, 275/275 with zero failed/blocking and exact manifest reconciliation;
- retained deliberate-failure diagnostic: `C:\tmp\sumi-pro00-launch-diagnostic\2026-07-31T10-24-27-613Z\partial-results.json`;
- production DB SHA-256 observed unchanged during implementation: `4166D749119B0EBB4B9ADF418EA18442FF6E0C14AE762147CD3D0FBE20F76459`;
- focused backend: 16 passed; focused frontend: 6 passed; manifest self-tests: 5 passed;
- full backend: 106 passed, 1 skipped; full frontend: 133 passed; lint and production build passed; `verify-v2.sh` passed;
- no accepted assertion or acceptance criterion was intentionally weakened in the implementation batch.

Pending before the DEV Reviewer gate:

- clean-machine rerun of the platform-appropriate standalone UAT wrapper and `verify-product`;
- exact final-run manifest/hash/count/reconciliation and artifact paths;
- visual review of the final-run 1440×1000 and 1280×800 screenshots;
- exact final-run production DB before/after hashes and post-run process/listener cleanup evidence;
- final diff/acceptance self-review, known limitations, and Reviewer checklist;
- independent Reviewer inspection and decision.
