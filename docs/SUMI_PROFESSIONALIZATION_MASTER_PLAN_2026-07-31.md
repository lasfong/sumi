# Sumi Professionalization Master Plan

Status: Canonical Post-V3 product program

Program date: 2026-07-31

Owner: Reviewer/orchestrator
Execution model: One approved vertical batch and one dedicated DEV task at a time

## 1. Purpose and authority

This document is the canonical roadmap for turning Sumi from a completed V3 development scope into a trustworthy professional personal workstation for Vietnamese equity investors and swing traders.

It governs all Post-V3 product work. The V3 plan, acceptance criteria, architecture decision, completed batch plans, and review reports remain historical evidence and required regression baselines. They do not override a newer verified defect or the acceptance requirements in this program.

The reports under `docs/tester/` are research inputs. Their claims are not release authority unless reproduced and mapped to retained evidence by an independent reviewer.

Every behavior-changing batch must:

- use an ExecPlan conforming to `PLANS.md`;
- preserve the repository invariants in `AGENTS.md`;
- map user-visible outcomes to immutable acceptance IDs in this document;
- deliver one complete vertical capability;
- retain deterministic machine-readable and browser evidence;
- stop at an independent reviewer gate;
- leave the product in a testable state.

Acceptance criteria may be extended between batches. They must not be weakened, deleted, reworded to hide a failure, or changed in the same implementation batch without explicit reviewer approval and a recorded decision.

## 2. Product target

### 2.1 Primary user

The Professional milestone targets one serious personal user who:

- studies and trades Vietnamese cash equities;
- primarily uses Daily and Weekly candles;
- practices historical decisions without future information;
- validates declarative strategies without writing arbitrary executable code;
- plans risk, records decisions, and reviews process quality;
- values local ownership and privacy over cloud collaboration.

### 2.2 Professional product outcome

At the Professional milestone, a user can:

1. maintain an auditable local market-data catalog;
2. start or resume a blind replay without future candles, signals, markers, metadata, or derived UI;
3. apply a useful released set of backend-authoritative technical indicators;
4. draw, plan risk, place simulated orders, and receive correct Vietnam-market feedback;
5. record a structured journal and checklist without losing chart context;
6. review trustworthy analytics that state sample and data limitations;
7. construct, compare, and reproduce declarative strategy experiments;
8. update supported market data through an explicit local-first workflow;
9. back up, restore, and verify the workstation without external services.

### 2.3 Explicit exclusions from the first Professional milestone

These are tracked rather than forgotten, but they are not release blockers:

- intraday candles and Vietnam derivatives;
- multi-chart or synchronized multi-timeframe layouts;
- arbitrary or sandboxed Python strategy execution;
- SaaS, authentication, accounts, collaboration, or cloud persistence;
- commercial distribution, licensing, billing, auto-update, and customer support;
- exhaustive parity with TradingView, AmiBroker, FireAnt, or every `pandas-ta` function.

The phrase "TradingView-like" remains prohibited. The phrase "professional local-first personal replay and technical-analysis workstation" may be used only after PRO-12 approval.

## 3. Verified baseline and reopened release status

### 3.1 Capabilities already present

- Backend-authoritative replay and indicator calculation.
- Manual replay, navigation, resume, orders, positions, T+2 feedback, journal, and checklist.
- Lightweight Charts v5 provider boundary.
- Indicator Manager released for EMA, RSI, MACD, CCI, and raw Volume.
- Versioned drawing system with Cursor, Horizontal, Trendline, Ray, Rectangle, Fibonacci Retracement, and Text.
- Declarative strategy evaluator shared by backtest and scanner.
- Scanner, Strategy Lab, Backtest, Analytics, Journal, and local data import surfaces.
- Local SQLite operation with temporary-database test and UAT support.

### 3.2 Corrected indicator inventory

The backend registry currently implements 15 definitions:

- SMA, EMA, MACD, RSI, Bollinger Bands;
- ATR, ADX, Ichimoku, Stochastic;
- Volume SMA, PSAR, SuperTrend, CCI, MFI, and Keltner Channels.

Only EMA, RSI, MACD, CCI, and frontend raw Volume are released through the V3 product UI. Backend availability is not product release. Every additional definition requires parameter, output-series, placement, styling, warm-up, persistence, no-future, and browser contracts.

Research clones under ignored `research_repos/` are references, not runtime integrations. The presence of Backtrader, backtesting.py, or many `pandas-ta` functions does not establish supported Sumi features.

### 3.3 Reopened P0 finding

Exploratory browser review found that a Scanner-created replay can begin before the selected signal while immediately displaying the future signal timestamp, price, strategy, regime, and entry metadata. Candle slicing may remain correct, but derived UI leaks future information and violates V3 R-01.

Therefore:

- historical V3 Batch 0-5 approvals remain recorded;
- current V3 release eligibility is conditional;
- Sumi must not be tagged, published, or described as Professional until PRO-00 is independently approved;
- future-information authorization must be enforced by backend response contracts, not by frontend hiding alone.

### 3.4 Evidence integrity gap

The current worktree includes:

- a modified `scripts/product-uat.mjs` that catches a missing historical baseline and returns `null`;
- untracked `docs/tester/`;
- an untracked `scripts/run-product-uat.ps1`.

Ignored `test-results/` paths cannot be the sole authority for a fail-closed UAT baseline. PRO-00 replaces that dependency with a checked-in manifest and records the disposition of the existing dirty state without reset, clean, deletion, staging, or unrelated rewriting.

## 4. Non-negotiable architecture and product invariants

### 4.1 Integrity and data

- Replay APIs and every derived response expose information only through the authorized replay boundary.
- Backend remains authoritative for indicator values, trading rules, accounting, and future-information authorization.
- Full scanner audit payload may be persisted locally but must not bypass a sanitized response view.
- Test and UAT processes use temporary databases and never mutate `backend/sumi.db`.
- Imports and synchronization are previewed, validated, and recoverable before accepted data changes.
- Dataset provenance includes source, timeframe, adjustment semantics, coverage, import/sync run, and transformation history.

### 4.2 Frontend

- React pages compose application capabilities; business logic does not migrate into route components.
- Chart-library calls remain behind Sumi provider adapters.
- Indicator state remains explicit and serializable: identity, definition, parameters, pane, visibility, style, and order.
- Drawing state remains versioned and independent of provider-native JSON.
- UI labels and pane semantics remain separate from backend dataframe column names.
- Forms, chart shortcuts, drawing shortcuts, and replay navigation have explicit focus ownership.

### 4.3 Backend

- Business rules remain outside FastAPI routes.
- Strategy evaluation remains declarative and does not use Python `eval` or arbitrary code execution.
- API contract changes use typed schemas and explicit compatibility behavior.
- Statistical values are not emitted as authoritative when required observations are absent.
- External data providers remain behind a local adapter and an approved source/license decision.

### 4.4 Dependencies and privacy

- No telemetry.
- No user strategy, journal, order, or trade data is transmitted externally.
- Market-data network access occurs only after an explicit user action.
- No new chart, drawing, analytics, or market-data dependency without spike evidence, license/security review, provider boundary, and recorded decision.

## 5. Professional acceptance contract

`PASS` requires implementation evidence, focused automated tests where appropriate, browser evidence for user-visible behavior, and no page/console/runtime errors. `N/A` requires reviewer approval; DEV may not assign it.

### 5.1 Global quality — PRO-G

| ID | Requirement |
| --- | --- |
| PRO-G-01 | Full backend tests, frontend tests, lint, production build, deterministic product UAT, and full product gate pass. |
| PRO-G-02 | Test and UAT runs use a temporary database; before/after SHA-256 proves `backend/sumi.db` is unchanged. |
| PRO-G-03 | Successful and failed UAT runs retain machine-readable results, manifest hash, runtime errors, HTTP failures, and required screenshots. |
| PRO-G-04 | A checked-in fail-closed assertion manifest prevents missing, duplicated, removed, or downgraded blocking assertions. |
| PRO-G-05 | No P0/P1 finding remains in the released workflow. |
| PRO-G-06 | No accepted V3 assertion is removed, renamed, weakened, or made non-blocking without a reviewer-approved migration. |
| PRO-G-07 | No unrelated working-tree change is included in a batch. Pre-existing changes are inventoried and preserved. |
| PRO-G-08 | The product remains local-first with no telemetry or external transmission of user trading, strategy, or journal data. |
| PRO-G-09 | Every behavior-changing batch updates its ExecPlan progress, decision, deviations, verification, rollback, and completion evidence. |
| PRO-G-10 | An independent reviewer inspects the diff, actual browser behavior, retained evidence, and production DB hash before approval. |

### 5.2 Replay and future-information integrity — PRO-INT

| ID | Requirement |
| --- | --- |
| PRO-INT-01 | Future candles are absent from APIs, chart series, websocket updates, and cached replay responses. |
| PRO-INT-02 | Future indicator values, warm-up artifacts, markers, orders, positions, scanner fields, and derived UI are absent before authorization. |
| PRO-INT-03 | Scanner replay creation explicitly supports `blind_practice` and `signal_review`; omitted intent defaults to `blind_practice`. |
| PRO-INT-04 | Blind practice returns `signal: null` before `reveal_at_index`, including create, get, resume, advance, rewind, and websocket flows. |
| PRO-INT-05 | At the reveal boundary, a versioned sanitized signal context appears exactly once without duplicate marker or state races. |
| PRO-INT-06 | Signal review starts at the signal candle, exposes context intentionally, and is visibly distinguished from blind practice. |
| PRO-INT-07 | Raw persisted scanner payload is never a frontend display authority and is not exposed through blind replay response contracts before reveal. |
| PRO-INT-08 | Legacy scanner sessions without replay intent behave as blind practice without destructive data migration. |
| PRO-INT-09 | Reload, rewind, resume, and repeated navigation recompute visibility from server-authoritative state. |
| PRO-INT-10 | Browser UAT proves before, at, and after boundary behavior using visible UI and captured response payloads. |

### 5.3 Backtest and analytics trust — PRO-BT

| ID | Requirement |
| --- | --- |
| PRO-BT-01 | Every run reports requested and actual date range, candle count, symbol coverage, warm-up, gaps, and excluded data. |
| PRO-BT-02 | Every result states execution timing, price basis, fees, taxes, slippage, liquidity assumptions, position sizing, and settlement rules. |
| PRO-BT-03 | Metrics include sample size, validity status, and an invalid/insufficient-data reason where applicable. |
| PRO-BT-04 | SQN is unavailable below 30 closed trades and is never synthesized by substituting a fake standard deviation. |
| PRO-BT-05 | Sharpe and Sortino use documented return periods; insufficient observations or downside samples produce `null` plus a reason. |
| PRO-BT-06 | Win rate and profit factor always show trade count and are not presented as positive evidence from an insufficient sample. |
| PRO-BT-07 | Zero-trade, one-trade, all-win, all-loss, flat-equity, missing-benchmark, and partial-coverage cases are honest and stable. |
| PRO-BT-08 | Hand-calculated deterministic ledgers verify PnL, fees, taxes, drawdown, benchmark, and valid statistical metrics. |
| PRO-BT-09 | Strategy Lab and Scanner consume the same metric validity contract and cannot rank invalid values as best. |
| PRO-BT-10 | A reproducibility manifest records strategy version, parameters, data identity, assumptions, engine version, and run timestamp. |

### 5.4 Daily workflow and localization — PRO-UX

| ID | Requirement |
| --- | --- |
| PRO-UX-01 | Dashboard shows data readiness, recent sessions, continue-practice, recent research runs, and actionable empty states. |
| PRO-UX-02 | Replay, Journal, and Analytics use a searchable session picker; normal use never requires a raw numeric Session ID. |
| PRO-UX-03 | Replay→trade→journal→analytics navigation preserves session and returns to the same workspace context. |
| PRO-UX-04 | Current symbol, timeframe, adjustment, date, bar index, OHLCV, mode, and data freshness are obvious without crosshair use. |
| PRO-UX-05 | Dates and numbers support Vietnamese semantics (`dd/MM/yyyy`, explicit timezone, unambiguous currency and percentages). |
| PRO-UX-06 | Loading, disabled, empty, partial, stale, and error states explain the user action required. |
| PRO-UX-07 | Core workflow remains usable at 1440×1000 and 1280×800; minimum width and mobile limitation are explicit. |
| PRO-UX-08 | Keyboard focus prevents chart, drawing, replay, trading, and form shortcuts from conflicting. |
| PRO-UX-09 | A user can complete a sustained daily practice workflow without internal implementation terminology or route-state loss. |

### 5.5 Data catalog, import, and synchronization — PRO-DATA

| ID | Requirement |
| --- | --- |
| PRO-DATA-01 | Data catalog reports source, symbols, timeframe, adjustment, first/last timestamp, row count, and last accepted update. |
| PRO-DATA-02 | Import preview reports parsed, rejected, duplicate, conflicting, missing, and out-of-order records before mutation. |
| PRO-DATA-03 | Ambiguous date, timezone, symbol, timeframe, or adjustment semantics block acceptance rather than guessing. |
| PRO-DATA-04 | Accepted imports are idempotent and retain a manifest, checksum, source filename, parser version, and outcome. |
| PRO-DATA-05 | Daily→Weekly aggregation has a deterministic market-calendar rule and provenance back to accepted Daily candles. |
| PRO-DATA-06 | Corrections never silently mix adjusted and unadjusted histories or overwrite conflicting accepted values. |
| PRO-DATA-07 | Backup/rollback restores the exact pre-import or pre-sync state. |
| PRO-DATA-08 | A market-data provider is integrated only after an ADR records terms, attribution, security, coverage, limits, and adjustment semantics. |
| PRO-DATA-09 | Synchronization is user-triggered, previewed, resumable, and reports partial provider failures without corrupting accepted data. |
| PRO-DATA-10 | No background network call, telemetry, or transmission of user trading data occurs during data synchronization. |

### 5.6 Indicators — PRO-IND

| ID | Requirement |
| --- | --- |
| PRO-IND-01 | Backend registry remains the calculation and parameter authority for every released indicator. |
| PRO-IND-02 | Frontend released definitions exhaustively map IDs to series, placement, formatter, scale, style, reference lines, and warm-up behavior. |
| PRO-IND-03 | Unknown or engine-only definitions cannot fall through to an incorrect EMA renderer. |
| PRO-IND-04 | Output columns are mapped by semantic series names rather than dataframe labels exposed as UI copy. |
| PRO-IND-05 | Overlay, oscillator, channel, histogram, marker, and benchmark-relative types have focused renderer contracts. |
| PRO-IND-06 | Multiple instances, editing, visibility, ordering, styles, persistence, reload, and resume work for every released definition. |
| PRO-IND-07 | Null/warm-up/gap data creates no invalid segment, crash, misleading active state, or future value. |
| PRO-IND-08 | Backend parity fixtures cover all released output series and representative edge cases. |
| PRO-IND-09 | Relative Strength aligns symbol and VNINDEX dates explicitly and reports unavailable benchmark coverage. |
| PRO-IND-10 | Ichimoku displacement is a documented display transform and cannot reveal a value calculated from a future candle. |
| PRO-IND-11 | Each indicator batch retains browser evidence for labels, values, scales, panes, settings, persistence, and replay navigation. |

### 5.7 Trade planning and journal — PRO-TRADE

| ID | Requirement |
| --- | --- |
| PRO-TRADE-01 | A plan records entry, stop, target, direction, account risk, planned quantity, fees, and expected R multiple. |
| PRO-TRADE-02 | Position sizing uses documented tick, lot, fee, tax, and available-cash rules with deterministic rounding. |
| PRO-TRADE-03 | Long/Short Risk-Reward drawing and trade plan share a versioned Sumi domain contract without provider-native persistence. |
| PRO-TRADE-04 | Planned, pending, filled, rejected, cancelled, settled, and closed states remain synchronized through replay and reload. |
| PRO-TRADE-05 | T+2 availability and rejection feedback state the blocked quantity, available quantity, and release date. |
| PRO-TRADE-06 | Checklist templates are editable, versioned, and captured as an immutable per-decision snapshot. |
| PRO-TRADE-07 | Journal taxonomy supports setup, regime, confidence, emotion, process mistake, rule violation, and review notes. |
| PRO-TRADE-08 | Review compares planned versus executed entry, stop, target, size, R, and outcome without hindsight leakage during practice. |
| PRO-TRADE-09 | Hand-calculated fixtures verify risk sizing, fees, taxes, T+2, partial availability, and R-multiple calculations. |
| PRO-TRADE-10 | Journal export and backup preserve local privacy and stable identifiers. |

### 5.8 Strategy research — PRO-STRAT

| ID | Requirement |
| --- | --- |
| PRO-STRAT-01 | Strategy parameters use typed product controls; users do not edit internal paths such as `indicators[1].length`. |
| PRO-STRAT-02 | Strategy definitions are versioned declarative data validated without `eval` or arbitrary Python execution. |
| PRO-STRAT-03 | Training and out-of-sample periods are explicit, non-overlapping, and retained in the run manifest. |
| PRO-STRAT-04 | Parameter sweeps enforce bounds, maximum variants, cancellation, and deterministic ordering. |
| PRO-STRAT-05 | Invalid/insufficient metrics cannot win ranking, heatmap, comparison, or recommendation surfaces. |
| PRO-STRAT-06 | Comparison shows coverage, sample size, assumptions, robustness, and out-of-sample results rather than only maximum PnL. |
| PRO-STRAT-07 | Saved strategy versions and runs can be reproduced or report why required data/version is unavailable. |
| PRO-STRAT-08 | Scanner, Replay, Backtest, and Strategy Lab share the same versioned strategy and indicator semantics. |

### 5.9 Release and operations — PRO-REL

| ID | Requirement |
| --- | --- |
| PRO-REL-01 | A 30-minute blind practice run completes with no missing core action, runtime error, or future-information leak. |
| PRO-REL-02 | Long-history replay with the released indicator/drawing load remains within recorded response, render, and memory budgets. |
| PRO-REL-03 | Backup and restored-copy verification cover sessions, drawings, indicators, plans, orders, trades, journal, strategies, and data manifests. |
| PRO-REL-04 | Accessibility covers keyboard access, focus visibility, names, form errors, modal focus, and core contrast. |
| PRO-REL-05 | Required evidence includes 1440×1000 and 1280×800 screenshots plus machine state for each released workflow. |
| PRO-REL-06 | Documentation states supported market, timeframes, adjustment semantics, rules, limitations, recovery, and privacy behavior. |
| PRO-REL-07 | No ignored local artifact is the sole release authority; selected manifests and evidence indexes are repository-controlled. |
| PRO-REL-08 | Release notes list exact scope, migrations, known limitations, evidence, rollback, and production DB hash. |
| PRO-REL-09 | An independent reviewer signs every batch and the final aggregate acceptance matrix. |
| PRO-REL-10 | Commit, tag, push, packaging, or publication requires separate explicit user authorization. |

## 6. Ordered development program

Only the next approved batch is authorized. A later batch may be refined using evidence from earlier work, but its outcome and acceptance contract cannot be silently reduced.

### Current program status — 2026-08-01

- The ordered program contains 13 batches, `PRO-00` through `PRO-12`.
- PRO-00 implementation is complete and pushed to `origin/master` in commit `55ec5f9` (`fix(replay): close PRO-00 integrity and evidence gaps`).
- Focused backend/frontend tests, the full technical suites, lint, build, `verify-v2`, and a temporary-database browser UAT result of 275/275 have passed during implementation.
- At the user's direction, the final canonical `run-product-uat`/`verify-product` rerun is separated from implementation and remains pending.
- PRO-00 has not passed the independent Reviewer gate. Its implementation-complete state is not a batch-complete or release-ready claim.
- PRO-01 through PRO-12 have not started. PRO-01 remains unauthorized until PRO-00 verification and independent review close successfully.
- Cross-machine continuation is governed by `docs/PROFESSIONALIZATION_HANDOFF_2026-08-01.md` and `docs/dev-prompts/PRO_00_VERIFICATION_CONTINUATION_PROMPT.md`.

### PRO-00 — Integrity and Evidence Closure

Outcome: Scanner-created replay is honest in blind and review modes, and product UAT cannot silently lose its baseline authority.

Implementation contract:

- add `replay_intent: blind_practice | signal_review`, defaulting to `blind_practice`;
- expose a versioned backend-sanitized `source_context`;
- keep full source audit data local while preventing premature API/frontend exposure;
- derive reveal state for create/get/resume/advance/rewind/websocket paths;
- label review mode and start it at the signal candle;
- add a checked-in UAT assertion manifest with fail-closed validation;
- retain manifest hash and actual assertion semantics in every result;
- record and preserve all pre-existing dirty files.

Primary acceptance: V3 R-01, G-01 through G-05, PRO-G-01 through PRO-G-10, PRO-INT-01 through PRO-INT-10.

Exit: independent browser and payload evidence proves before/at/after behavior; all gates pass; production DB hash is unchanged.

### PRO-01 — Backtest and Analytics Trust

Outcome: quantitative output communicates what the data supports and refuses false precision.

Implementation contract:

- introduce typed `DataCoverage`, `ExecutionAssumptions`, `MetricResult`, and `RunManifest` response structures;
- use nullable metric values with `valid`, `insufficient_data`, or `not_applicable` status and a reason;
- require at least 30 closed trades for SQN;
- require at least 30 periodic returns for Sharpe/Sortino and at least two downside observations for Sortino;
- show sample size beside rate/ratio metrics and neutralize insufficient-result styling;
- propagate validity through Backtest, Analytics, Strategy Lab, and Scanner ranking.

Primary acceptance: PRO-BT-01 through PRO-BT-10 and regression PRO-INT.

Exit: hand-calculated fixtures and browser scenarios cover misleading small-sample cases.

### PRO-02 — Daily Trader Workflow

Outcome: the product behaves as one workstation instead of disconnected pages.

Implementation contract:

- replace the empty landing page with data readiness, recent sessions, continue actions, and research history;
- introduce a shared searchable session picker for Replay, Journal, and Analytics;
- persist the selected session in route state and recover it on reload;
- standardize Vietnamese date, number, currency, timezone, and empty/error copy;
- remove normal dependence on raw IDs and raw internal parameter paths from user surfaces.

Primary acceptance: PRO-UX-01 through PRO-UX-09.

Exit: a browser journey completes Dashboard→Replay→Journal→Analytics→Replay without lost context.

### PRO-03 — Data Catalog and Import Quality

Outcome: the user knows exactly what data is present and can safely import Daily/Weekly histories.

Implementation contract:

- add catalog and import-run domain records without mixing business rules into routes;
- implement preview, conflict classification, manifest, checksum, idempotence, and rollback;
- keep adjusted/unadjusted histories isolated;
- define Weekly aggregation from accepted Daily candles using explicit Vietnam trading dates and provenance.

Primary acceptance: PRO-DATA-01 through PRO-DATA-07.

Exit: malformed and conflicting fixture sets cannot silently mutate accepted data.

### PRO-04 — Core Indicator Expansion

Outcome: release SMA, Bollinger Bands, ATR, and backend Volume SMA as complete product capabilities.

Implementation contract:

- replace unsafe renderer fallback with an exhaustive released-definition catalog;
- define semantic output mapping and placement for each definition;
- distinguish raw Volume from Volume SMA;
- preserve backend calculation authority and full Indicator Manager persistence.

Primary acceptance: PRO-IND-01 through PRO-IND-08 and PRO-IND-11.

### PRO-05 — Momentum and Relative Strength

Outcome: release MFI, Stochastic, ADX, and Relative Strength vs VNINDEX.

Implementation contract:

- add Relative Strength as an explicit backend definition;
- define benchmark alignment, missing-day, missing-index, and warm-up behavior;
- provide oscillator scale/reference and multi-series semantics.

Primary acceptance: PRO-IND-01 through PRO-IND-09 and PRO-IND-11.

### PRO-06 — Advanced Trend Overlays

Outcome: release Keltner Channels, PSAR, and SuperTrend.

Implementation contract:

- define channel, point/marker, direction, and trend-color semantics;
- verify gaps, nulls, scale behavior, navigation, and persistence.

Primary acceptance: PRO-IND-01 through PRO-IND-08 and PRO-IND-11.

### PRO-07 — Ichimoku Contract

Outcome: release Ichimoku without look-ahead or misleading displacement.

Implementation contract:

- isolate calculation timestamp from display timestamp;
- document Tenkan, Kijun, Chikou, and cloud displacement;
- clip every displayed value to information available at the replay boundary;
- test start/end boundaries, gaps, warm-up, replay, rewind, and resume.

Primary acceptance: all PRO-IND, especially PRO-IND-10.

### PRO-08 — Trade Planning and Journal

Outcome: transform replay decisions into a measurable risk and learning process.

Implementation contract:

- add versioned trade-plan and checklist-template schemas;
- add position sizing and R-multiple domain services;
- add a Sumi-owned Long/Short Risk-Reward drawing document;
- link plan, order, execution, position, and journal without duplicating accounting authority;
- make market-rule versions explicit and configurable.

Primary acceptance: PRO-TRADE-01 through PRO-TRADE-10.

### PRO-09 — Strategy Research UX

Outcome: strategy comparison is safe, understandable, and resistant to overfitting.

Implementation contract:

- replace raw paths with typed controls generated from validated strategy metadata;
- add immutable versions, train/test ranges, cancellation, bounded sweeps, and reproducibility;
- prevent invalid metrics from ranking or recommendation.

Primary acceptance: PRO-STRAT-01 through PRO-STRAT-08 and PRO-BT regression.

### PRO-10 — Market Data Provider Decision

Outcome: make an evidence-backed approve/reject decision before network integration.

Mandatory spike evidence:

- source terms, license, attribution, and redistribution constraints;
- authentication and secret handling;
- Daily/Weekly symbol and historical coverage;
- timestamp, timezone, corporate-action, and adjustment semantics;
- throttling, failure, correction, and reproducibility behavior;
- provider boundary, security review, bundle/runtime impact, and fallback.

Exit:

- approved ADR unlocks PRO-11; or
- rejection records why and retains file import as the supported path.

### PRO-11 — One-Click Data Sync

Outcome: explicit user-triggered synchronization updates accepted local data safely.

This batch is unauthorized unless PRO-10 approves a provider.

Implementation contract:

- provider adapter, preview, explicit confirmation, progress, retry/resume, manifest, and rollback;
- no scheduled/background network traffic;
- no transmission of private workstation data.

Primary acceptance: PRO-DATA-08 through PRO-DATA-10 plus PRO-DATA regression.

### PRO-12 — Professional Release Hardening

Outcome: produce an evidence-backed Professional release candidate for sustained personal use.

Implementation contract:

- run sustained blind practice and cross-feature regression;
- measure long-history render, API response, memory, and repeated mount/unmount behavior;
- verify backup/restore on a restored copy;
- complete accessibility, keyboard, supported-platform, privacy, and recovery documentation;
- publish an aggregate acceptance matrix and known limitations.

Primary acceptance: every released V3 and PRO acceptance ID.

Exit: independent reviewer approval. Tag, commit, push, package, or publish remains separately authorized.

## 7. Batch lifecycle and task strategy

1. Reviewer frames exactly one batch and creates a standalone prompt under `docs/dev-prompts/`.
2. A new DEV task opens the current checkout and branch.
3. DEV audits current state and writes the batch ExecPlan before product code.
4. DEV implements, verifies, self-reviews, updates evidence, and stops.
5. Reviewer independently inspects the diff and actual browser behavior.
6. Reviewer either approves and closes the batch or returns a bounded hardening prompt.
7. Only approval unlocks the next prompt.

There is one writer at a time. The reviewer must not edit overlapping files while DEV is active. A branch or worktree is used only when explicitly authorized for isolated parallel writes.

Every DEV handoff includes:

- outcome and acceptance IDs;
- exact changed files;
- architecture decisions and deviations;
- exact commands and counts;
- UAT artifact path and reviewed screenshots;
- production DB before/after hashes;
- known failures, limitations, and reviewer checks;
- explicit confirmation that no acceptance test was weakened.

## 8. Verification and evidence standard

Every behavior-changing batch runs, as applicable:

```bash
git diff --check
cd backend && pytest
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
```

Windows verification must also record:

```powershell
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

User-facing work requires:

- focused assertions for the new behavior;
- preservation of every accepted baseline assertion;
- visible browser actions rather than hidden state injection;
- response-payload inspection for integrity contracts;
- screenshots at 1440×1000 and 1280×800;
- captured console, page, provider, API, and persistence errors;
- machine-readable results for both success and failure.

## 9. Rollback and compatibility policy

- Every persisted schema is versioned and has a fixture for the previous accepted version.
- Read compatibility is preferred before write migration.
- Destructive migration requires backup, restored-copy verification, and reviewer approval.
- A failed batch is reverted only through an explicit, target-verified action; never by broad reset or clean.
- Feature flags are allowed only when they preserve honest user-visible state and do not hide incomplete acceptance.
- External-provider failure must leave local file import and accepted local data operational.

## 10. Program risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Future information leaks through non-candle metadata | Backend-sanitized view, boundary tests for every response path, browser payload assertions. |
| Green UAT after baseline disappearance | Checked-in fail-closed manifest, manifest hash, duplicate/removal/downgrade tests. |
| Statistical false confidence | Typed metric validity, minimum samples, neutral UI, hand-calculated fixtures. |
| Indicator count expands faster than product quality | Released-definition catalog and bounded indicator batches. |
| Ichimoku creates look-ahead through displacement | Isolated batch with calculation/display timestamp contract. |
| Data imports silently mix incompatible histories | Provenance, preview, adjustment isolation, conflict quarantine, rollback. |
| Provider terms or behavior are unsuitable | Mandatory decision spike before integration. |
| Strategy Lab encourages overfitting | Train/test split, bounded sweeps, robustness and invalid-metric exclusion. |
| Long-running DEV task drifts across scope | New DEV task and reviewer gate for every batch. |
| Dirty user files are lost | Initial inventory, no reset/clean/delete, one-writer rule. |

## 11. Deferred Post-Professional backlog

The following items require new product decisions after PRO-12:

- intraday storage, sessions, replay, aggregation, and performance;
- VN30 derivatives contract rollover and market rules;
- dual/multi-chart synchronized time and crosshair;
- portfolio-level capital allocation and pyramiding;
- sector and fundamental scanner filters;
- Volume Profile, VWAP, and additional drawing geometry;
- sandboxed custom Python;
- packaging, updater, commercial license, support, and distribution;
- optional PostgreSQL/background workers only when measured SQLite/local-run limits justify them.

They may not be pulled into an active Professional batch without reviewer re-planning.

## 12. Version and release policy

- PRO-00 is the earliest candidate for a V3.0.1 integrity release, but no version action is implied.
- PRO-01 through PRO-11 build the Professional capability set.
- PRO-12 produces the Professional release-candidate decision.
- Version numbers, commits, tags, pushes, packages, and publication require separate explicit user authorization.
