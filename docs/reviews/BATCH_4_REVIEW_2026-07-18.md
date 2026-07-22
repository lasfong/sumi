# Batch 4 Reviewer gate — 2026-07-18

## Verdict

Status: **RETURNED FOR ONE BOUNDED CLOSURE. Batch 4 remains open and Batch 5 is unauthorized.**

The integrated PracticeRail, as-of lifecycle projection, execution-derived markers, T+2 feedback, journal surface, responsive layout, and ordinary single-step workflow are materially implemented. The submitted 246-ID artifact is internally consistent and the existing technical gates pass. Batch 4 cannot close because the Reviewer independently reproduced a ledger-changing navigation defect and an exact-context validation defect that the current tests and UAT do not cover.

Acceptance classification:

| Acceptance | Reviewer verdict | Reason |
| --- | --- | --- |
| T-01 | Accepted for this gate | Wide/compact geometry and the explicit sub-1180 limitation are visible and machine-checked. |
| T-02 | FAIL | A pending LIMIT can miss a valid intermediate fill when replay advances by more than one candle. |
| T-03 | PARTIAL | Journal/checklist persistence works, but backend validation accepts a date that does not belong to the declared current candle. |
| T-04 / R-04 | FAIL | `+5` is not semantically equivalent to five single steps for pending-order execution and projected lifecycle state. |
| T-05 | PARTIAL | The ordinary UI path is proven, but a core visible navigation action can produce an incorrect trade ledger. |
| R-02 | PARTIAL | Displayed contexts agree in the happy path; the journal authority does not enforce the exact candle date. |
| G-01–G-03, G-05 | Accepted for this gate | Existing tests, lint, build, artifact retention, temporary DB and local-first boundaries pass inspection. |
| G-04 | FAIL | Two unresolved P1 findings remain. |

## Independent evidence

- Provenance remained `master` at `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2^{}` remained `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`. Reviewer performed no branch/worktree/stage/commit/push/merge/reset/clean/checkout/tag operation.
- Focused backend workflow/lifecycle tests: 19 passed. Full backend: 81 passed / 1 skipped. Complete frontend: 20 files / 113 tests passed; lint and production build passed.
- `./scripts/verify-v2.sh`: passed. `git diff --check`: passed.
- Submitted full-gate result `test-results/product-uat/2026-07-18T07-22-47-858Z/results.json` is internally consistent: 246 checks, 246 unique IDs, 246 passed, 0 failed, `blockingFailed: 0`, 22 additive `batch4.*` IDs, empty runtime/provider/indicator-request failure arrays, and one separately classified expected T+2 rejection.
- Reviewer inspected the submitted Batch 4 wide, T+2, closed-state, reload, and 1280×800 screenshots. The chart remains primary, the rail is contained, context and rejection feedback are legible, and no visual blocker was found.
- Production DB SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d` after review commands.

Green existing gates do not override the counterexamples below because neither counterexample is represented in the current suite.

## Findings

### B4-R01 — P1 — Multi-step advance skips valid intermediate LIMIT fills

`ReplayService.next_candle()` first moves `current_index` directly by `steps`, then calls `_match_pending_orders()` once. That matcher reads only the final visible candle. If a LIMIT is touched on an intermediate crossed candle but not on the destination candle, no execution is created. This makes `+5` observably different from five `Next` actions and can corrupt order status, cash, position, P&L, T+2 availability, markers, and subsequent CLOSE behavior.

Reviewer reproduction used four in-memory candles with closes `100, 105, 120, 130`, placed a valid BUY LIMIT at `105` on index 0, and called `next_candle(..., steps=3)`. The order crossed an eligible candle at index 1 but the result at index 3 was `executions: 0`.

Current UAT does not test this contract. Its LIMIT path loops over the single-step `Next` button; it never uses `+5` with an intermediate-only hit even though the ExecPlan explicitly claims step/±5 synchronization.

Required closure:

- make an N-step advance lifecycle-equivalent to N chronological single steps for every crossed candle, including earliest eligible LIMIT fill, execution price/candle identity, cash, position, P&L, T+2, bankruptcy/MTM semantics, status and marker projection;
- preserve the non-destructive rewind/forward rule: a retained execution reappears exactly once and is never re-executed;
- cover ordinary destination, completion-boundary, no-hit and intermediate-hit paths, including equality of one `+5` operation and five single steps;
- do not solve this in the UI or by weakening/removing the `+5` control.

### B4-R02 — P1 — Checklist date is client-trusted rather than bound to the authoritative candle

`JournalService._validate_checklist()` checks session ID, symbol and current index, but accepts any nonempty date string. The Reviewer created a checklist for a session whose current candle was `2024-01-02` with context date `2099-12-31`; the backend accepted and stored it. This contradicts the recorded exact-context envelope and allows a journal entry to claim a different market date from the header/trade workspace.

Required closure:

- resolve the authoritative current candle using the session symbol, timeframe, adjustment, date range and current index, and require the checklist date to equal its canonical date representation;
- reject malformed, impossible, stale and merely nonempty wrong dates without mutating journal state;
- keep legacy opaque notes readable and avoid a migration or rewrite of existing records;
- add focused service/API tests for correct date, wrong valid date, impossible date, stale index and cross-session/symbol context.

### B4-R03 — P1 — UAT overstates ±5 and exact journal-context coverage

The 22 additive IDs prove the single-step happy path, but no Batch 4 ID establishes an intermediate LIMIT fill through the visible `+5` action. The journal check asserts `candleIndex` only and never proves backend rejection of a mismatched date. Therefore the machine evidence cannot detect B4-R01 or B4-R02 while reporting R-02/R-04/T-02–T-05 green.

Required closure:

- retain all 246 existing IDs, names, pass values and blocking classification;
- add blocking browser evidence that a pending LIMIT touched only on an intermediate candle executes on the correct earliest candle through the visible `+5` path and remains exactly-once through rewind/forward/reload;
- add observable API/UI evidence that a mismatched journal date is rejected without a saved row, while a correct current-context checklist saves and reloads;
- compare the closure result to the accepted 246-ID baseline for missing, duplicate, renamed, changed-pass and changed-blocking IDs.

## Scope protection

Keep the Batch 4 architecture: one practice snapshot authority, service-layer lifecycle logic, non-destructive as-of projection, execution-derived markers, existing journal envelope v1, PracticeRail, Lightweight Charts provider boundary, IndicatorEngine, Sumi drawing domain, temporary UAT database and local-first behavior.

Do not add event sourcing, alternate timelines, order amend/cancel, short selling, a database migration, a new dependency, telemetry, product-wide redesign, acceptance weakening, production DB mutation, or Batch 5 work.

Use `docs/dev-prompts/BATCH_4_REVIEW_CLOSURE_PROMPT.md` as the only DEV authority for this bounded continuation. Stop again at Reviewer gate.

## Final Reviewer closure — 2026-07-18

Status: **APPROVED AND CLOSED. Batch 5 is authorized only through its dedicated standalone DEV prompt.**

B4-R01–B4-R03 are closed. The Reviewer inspected the closure diff, focused corpus, additive UAT and required screenshots, then independently reran the technical and product gates.

### Closure determination

- **B4-R01 closed:** multi-step advance now applies cursor persistence, MTM/equity/bankruptcy and pending-order matching to each crossed candle in chronological order. Focused tests prove the earliest intermediate fill, multi-step/single-step state equality, destination/no-hit/completion boundaries, bankruptcy equivalence and retained execution identity after rewind.
- **B4-R02 closed:** checklist schema v1 now accepts only canonical ISO `YYYY-MM-DD` equal to the authoritative current session candle. Wrong valid, malformed, impossible, noncanonical, stale and cross-workspace contexts are rejected before insertion; legacy opaque notes remain readable and no migration was introduced.
- **B4-R03 closed:** the visible `+5` path proves earliest intermediate execution date/index/price, cash, position, T+2 and marker state, plus exactly-once rewind/forward/reload. The mismatched-date rejection is contained and proved to leave journal IDs unchanged; the correct checklist persists and reloads.

### Independent final evidence

- Focused backend closure/workflow/lifecycle/API: 40 passed. Full backend: 97 passed / 1 skipped. Frontend: 20 files / 113 tests passed; lint and production build passed.
- `./scripts/verify-v2.sh`: passed. `git diff --check`: passed.
- Independent standalone UAT: `test-results/product-uat/2026-07-18T15-22-01-472Z/results.json` — 254 passed, 0 failed, `blockingFailed: 0`.
- Independent full product gate: `test-results/product-uat/2026-07-18T15-23-28-222Z/results.json` — 254 passed, 0 failed, `blockingFailed: 0`; `./scripts/verify-product.sh` passed.
- Both independent artifacts have empty `runtimeErrors`, `providerErrors`, and `indicatorRequestFailures`. The expected HTTP 400 T+2 and HTTP 409 mismatched-date console messages are bounded to their deliberate rejection windows.
- Comparison with the accepted 246-ID baseline: missing `[]`, duplicates `[]`, changed pass `[]`, changed blocking classification `[]`, and exactly eight additive passing blocking `batch4.closure.*` IDs.
- Reviewer inspected the wide intermediate-fill, journal reload and compact 1280×800 screenshots. The execution is visibly recorded on bar 65 after reaching bar 69, the authoritative bar/date journal entries survive reload, and the chart/rail remain contained.
- Production DB SHA-256 remained `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`. Provenance remained `master` / HEAD `108aa5dc0e26994607836e2b3b33f482e3791b4e`; `v2.0.0-rc2^{}` remained `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e`.

### Scope and release boundary

This approval closes the **Batch 4 Integrated Trading-Practice Workflow**. It does not yet declare V3 release-ready, product-complete, or a professional manual replay product. Batch 5 must still complete sustained-session, performance/memory, backup/restore/migration, accessibility/keyboard, documentation and full-release acceptance evidence. “TradingView-like” remains prohibited.

Batch 5 may begin only in a separate DEV task using `docs/dev-prompts/BATCH_5_PRODUCT_HARDENING_V3_RC_PROMPT.md`. No release tag, commit, push or production DB mutation is authorized.
