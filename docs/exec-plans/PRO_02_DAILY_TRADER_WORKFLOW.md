# PRO-02 — Daily Trader Workflow

## Outcome

Turn the disconnected Dashboard, Replay, Journal, and Analytics routes into one coherent local-first daily practice workflow. A user can discover data/session readiness, continue or choose a replay through product UI, move Replay → Journal → Analytics → Replay without typing a raw numeric session ID, and keep the same session/context through route changes and reload.

This batch implements only `PRO-UX-01` through `PRO-UX-09` and preserves all accepted V3, PRO-00, and PRO-01 assertions. It does not start PRO-03 data-catalog/import semantics.

## Preconditions

- PRO-01 is independently approved and committed locally.
- Worktree has no unexpected staged, unstaged, or untracked changes overlapping PRO-02.
- `backend/sumi.db` SHA-256 is recorded before work and is never used by automated verification.
- No dependency, migration, branch, worktree, commit, push, tag, release, or publication is authorized.

If a precondition is false, record one blocker in `docs/AUTONOMOUS_EXECUTION_STATE.md` and stop without implementation.

## Acceptance mapping

| ID | Required product evidence |
| --- | --- |
| PRO-UX-01 | Dashboard shows data readiness, recent sessions, continue actions, recent research runs, and actionable empty/error/loading states. |
| PRO-UX-02 | Replay, Journal, and Analytics share a searchable session picker; normal use never requires a raw numeric ID. |
| PRO-UX-03 | Replay → Journal → Analytics → Replay preserves selected session and replay context. |
| PRO-UX-04 | Replay visibly retains symbol, timeframe, adjustment, date, bar index, OHLCV, mode/intent, and readiness/freshness context. |
| PRO-UX-05 | Shared Vietnamese date/number/currency semantics use `vi-VN`, explicit `Asia/Ho_Chi_Minh`, `dd/MM/yyyy`, and unambiguous VND/percent labels. |
| PRO-UX-06 | Loading, disabled, empty, partial, stale, and error states explain the next user action. |
| PRO-UX-07 | Core workflow is usable at 1440×1000 and 1280×800; existing minimum-width/mobile limitation remains explicit. |
| PRO-UX-08 | Session search/selection and navigation do not conflict with Replay/chart/drawing/form keyboard ownership. |
| PRO-UX-09 | Deterministic browser journey completes without internal IDs/paths as required user input, route-state loss, or runtime errors. |

Regression authority includes V3 `G-01`–`G-05`, `R-01`–`R-05`, `T-01`–`T-05`, all accepted indicator/drawing IDs, `PRO-INT-01`–`PRO-INT-10`, and `PRO-BT-01`–`PRO-BT-10`.

## In scope

- Replace the placeholder root route with a Dashboard page.
- Reuse existing `/api/symbols`, replay-session list, and Strategy Lab history APIs for readiness/recent content; add a narrow typed read-only summary service/endpoint only if repository discovery proves existing APIs cannot provide a required deterministic state.
- Shared searchable `SessionPicker` component and typed session-display model.
- Canonical selected-session synchronization between URL query (`session`), persisted replay store, and route pages.
- Continue/recent-session actions and actionable empty/loading/error states.
- Remove raw numeric Session ID inputs from normal Journal and Analytics workflows.
- Session-preserving product navigation among Replay, Journal, and Analytics.
- Shared Vietnamese formatting utilities without a new dependency.
- Focused tests, additive fail-closed UAT assertions, screenshots, evidence, and handoff.

## Out of scope

- PRO-03 catalog records, import preview/conflict classification, provenance manifests, Weekly aggregation, sync, or provider work.
- Journal taxonomy/template expansion (PRO-08), metric changes (PRO-01), new indicators, drawing tools, or strategy UX.
- Broad backend route/business-logic rewrites, schema migration, authentication, telemetry, or external network access.
- Commit/push/release and PRO-03 planning/implementation.

## Target architecture

```text
App routes
  -> DashboardPage
      -> symbols readiness query
      -> recent replay sessions query
      -> recent Strategy Lab runs query
  -> shared SessionSelectionController
      -> URL ?session=<id>
      -> persisted replayStore.sessionId
      -> validated session list
  -> SessionPicker
      -> ReplayPage / JournalPage / AnalyticsPage
  -> shared Vietnamese format utilities
```

Route components remain composition surfaces. Session selection/parsing/formatting belongs in shared hooks/utilities, not duplicated page code. Existing Replay controller remains the Replay application authority.

## Task breakdown and DoD

### T02-01 — Initial audit and contracts

Tasks:

- Read all authority named in the standalone prompt.
- Record branch/HEAD/origin/tag, status inventory, diff check, production DB hash, runtime versions, retained PRO-01 evidence, and available relevant tests.
- Audit App/Sidebar routes, replay store, Replay setup/controller, Journal/Analytics raw-ID paths, symbols/session/research APIs, responsive CSS, keyboard policy, and current UAT manifest.
- Define typed `SessionSummary`, selection precedence, invalid/missing query behavior, Vietnamese formatting contract, and no-migration compatibility.

DoD:

- ExecPlan current-architecture and decision logs contain no unresolved discoverable decision.
- State ledger milestone is `framing`.
- No product code changed before this record.

### T02-02 — Shared session context and picker

Tasks:

- Build one reusable searchable session picker using server-returned sessions.
- Display symbol, timeframe, adjustment, status, date range, current bar/index, and honest blind/review mode where available.
- Implement URL/store precedence: a valid URL session wins and updates the store; otherwise a valid persisted session is used; invalid/missing sessions show an actionable picker and never silently fall back to ID 1.
- Provide helpers for session-preserving route links.
- Preserve keyboard ownership: typing/search/select controls must not trigger replay/drawing shortcuts.

DoD:

- Focused tests cover search, keyboard selection, valid URL, persisted fallback, invalid URL, empty list, loading/error, and no numeric-ID requirement.
- Reload/resume keeps the same valid session.
- No business logic moves into route components.

### T02-03 — Dashboard

Tasks:

- Replace placeholder `/` route with `DashboardPage`.
- Show local data readiness from existing symbols/data availability, recent replay sessions with Continue actions, and recent saved Strategy Lab runs.
- Provide clear actions for no data, no sessions, no research, partial query failure, and retry/navigation.
- Do not claim PRO-03 catalog/provenance coverage.

DoD:

- Dashboard focused tests cover ready, empty, loading, partial failure, and complete error states.
- Continue action selects the session and navigates to Replay with canonical route state.
- Research actions navigate to the existing Strategy Lab surface without inventing unsupported resume semantics.

### T02-04 — Integrated Replay/Journal/Analytics navigation

Tasks:

- Add product-visible session-preserving links/tabs/actions among Replay, Journal, and Analytics.
- Integrate shared picker into Replay setup/resume, Journal, and Analytics.
- Remove raw numeric Session ID input from normal Journal/Analytics UX.
- Preserve current replay index and workspace state; navigation must not create/reset a session.
- Invalid/deleted sessions lead to an actionable picker, not an opaque API error.

DoD:

- Focused route tests prove Dashboard → Replay → Journal → Analytics → Replay retains one session ID and current replay index.
- Direct-load and browser reload with `?session=` restore correctly.
- Back/forward navigation does not lose selection or create duplicate sessions.

### T02-05 — Vietnamese semantics and state copy

Tasks:

- Add dependency-free shared formatters for market date, timestamp, VND, decimal, integer volume, and percent using `vi-VN` and `Asia/Ho_Chi_Minh`.
- Replace user-visible ambiguous formatting only within Dashboard/Replay/Journal/Analytics and shared picker scope.
- Make timeframe, adjustment, mode/intent, and timezone explicit where context could be ambiguous.
- Standardize actionable loading/disabled/empty/partial/stale/error copy.

DoD:

- Deterministic formatter tests cover zero, negative values, null/unavailable values, timezone boundary, and `dd/MM/yyyy` output.
- UI does not expose implementation names such as raw query keys, internal paths, or require numeric IDs.

### T02-06 — Responsive/accessibility hardening

Tasks:

- Verify 1440×1000 and 1280×800 layouts for Dashboard and cross-route workflow.
- Ensure picker labels, focus order, visible focus, button names, loading/status roles, and error actions are accessible.
- Preserve the existing explicit mobile/minimum-width limitation.

DoD:

- Focused accessibility tests cover names, focus, picker keyboard interaction, and shortcut isolation.
- No horizontal loss of core session/navigation actions at required desktop viewports.

### T02-07 — Additive product UAT and evidence

Tasks:

- Extend the checked-in UAT manifest additively with stable blocking `pro02.*` IDs mapped to `PRO-UX-01`–`PRO-UX-09`.
- Preserve the sealed V3/PRO-00 baseline and all PRO-01 IDs; add negative self-tests proving removal/rename/blocking downgrade of prior accepted IDs fails closed.
- Use visible UI actions with deterministic temporary data for the complete route journey.
- Capture at minimum `pro02-dashboard-1440x1000.png` and `pro02-cross-route-workflow-1280x800.png`.

Required browser assertions:

- dashboard readiness/recent sessions/research and empty/error action contracts;
- searchable picker and no raw numeric ID field on Journal/Analytics;
- exact selected session across Dashboard → Replay → Journal → Analytics → Replay;
- replay index unchanged across route round trip;
- URL/reload restoration;
- Vietnamese date/VND/timezone labels;
- compact layout, keyboard isolation, and no console/page/provider/request failure;
- no-future and PRO-00/PRO-01 regression assertions remain green.

DoD:

- Standalone UAT and `verify-product` pass with exact manifest reconciliation.
- Screenshots are visually reviewed, not merely present.
- Result includes manifest hash/counts, errors, temporary DB identity, production hashes, origins, and screenshot metadata.

### T02-08 — Self-review and Reviewer handoff

Tasks:

- Inspect complete diff and untracked inventory against tasks/acceptance IDs.
- Record deviations, known limits, rollback, exact commands/counts/durations, artifact paths/hashes, DB hashes, and cleanup.
- Update `docs/AUTONOMOUS_EXECUTION_STATE.md` to `reviewer-gate` with one exact next action.

DoD:

- No PRO-03 work, migration, dependency, telemetry, external transmission, commit, or push.
- Explicit statement confirms no accepted assertion was weakened.
- DEV stops for independent Reviewer; it does not ask the user what to do next beyond the single Reviewer action recorded in the state ledger.

## Verification sequence

Discover exact focused paths during T02-01, then run in this order:

```powershell
git diff --check
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

```text
focused frontend session/dashboard/route/format/accessibility tests
focused backend tests only if a narrow read-only summary contract is added
node --test scripts/product-uat-manifest.test.mjs
full backend pytest
full frontend test suite
frontend lint
frontend production build
./scripts/verify-v2.sh
powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1
./scripts/verify-product.sh
```

After gates: repeat DB hash, `git status --short --branch`, `git diff --check`, complete diff review, ports/process cleanup, temporary DB absence, result reconciliation, and screenshot visual review.

## Rollback

PRO-02 should be removable by reverting only its bounded Dashboard/session-context/formatting/tests/UAT/docs changes. It must not rewrite persisted replay sessions or require a database migration. Any rollback is an explicit Reviewer-authorized target operation; never use broad reset/clean.

## Progress log

- 2026-08-02: Strong-model framing package created after PRO-01 independent approval. PRO-01 was committed locally as `b3f18d8`; implementation has not started.

## Decision log

- Use URL query plus persisted replay store as the shared session identity; server session state remains authoritative.
- Reuse existing symbols, replay-session, and Strategy Lab history APIs before considering a narrow read-only summary endpoint.
- Keep PRO-02 readiness intentionally shallower than PRO-03 data catalog/provenance.
- Use dependency-free `Intl` formatting with explicit Vietnamese locale/timezone.
- Prepare only the next batch in detail; later prompts are framed after prior Reviewer evidence to avoid stale architecture assumptions.

## Completion evidence

Pending implementation. Do not mark complete before all DoD items and independent Reviewer approval.
