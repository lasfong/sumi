# PRO-02 Reviewer Rework 01 — session authority, honest readiness, and evidence closure

You are continuing the existing PRO-02 DEV batch after an Independent Reviewer verdict of `REWORK`. Read this file completely, then re-read `AGENTS.md`, `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`, `docs/AUTONOMOUS_EXECUTION_STATE.md`, the PRO-02 prompt, and the complete PRO-02 ExecPlan. Preserve the retained 293-check artifact as failure/rework history. Do not start PRO-03, commit, push, add a dependency, migrate data, or mutate `backend/sumi.db`.

Work continuously through all findings below, focused regression tests, standalone UAT, the exact full product gate, evidence updates, and return to the Independent Reviewer gate. Do not ask routine questions or report per-file progress.

## Reviewer verdict

Current PRO-02 is not approved. There is no new P0/no-future leak, but all findings below are blocking for the batch.

## R02-01 — Implement one validated URL/store/session authority

Current defects:

- `useSessionSelection` treats any positive numeric URL ID as valid without verifying it exists.
- `ReplayPage` does not use the selection controller. Dashboard Continue changes only the URL, so a clean store does not open that session.
- Replay picker/resume changes only the store, not the canonical URL. URL and store can disagree and reload can restore the wrong session.
- Deleted/invalid URL or persisted IDs lead to opaque API errors rather than an actionable picker.

Required correction:

- Use one shared validated selection controller on Replay, Journal, and Analytics.
- Validate URL/store IDs against the authoritative session list or a focused session GET before activating them.
- Precedence: valid URL wins and synchronizes store; otherwise a valid persisted ID is used and canonicalized into the URL; invalid syntax, missing/deleted ID, or failed validation clears invalid selection and shows an actionable picker.
- Dashboard Continue, SessionSetup Resume, Replay picker selection, Journal picker, and Analytics picker must all update URL and store together.
- Direct load, reload, browser back, and browser forward must restore the canonical session without creating/resetting it.

Required focused tests:

- valid URL wins;
- valid persisted fallback canonicalizes URL;
- invalid syntax does not fall back silently;
- positive but deleted/missing ID is rejected;
- selection updates URL and store;
- Replay direct load uses the URL session;
- Dashboard Continue opens the selected session from an initially empty store;
- reload/back/forward preserve the same session;
- round trip preserves authoritative current index.

## R02-02 — Make Dashboard readiness honest

Current defect: `/api/symbols` metadata alone is labeled `Data Ready`; a symbol with zero candles would pass, and fallback exchange/asset labels invent metadata.

Required correction:

- Implement the narrow typed read-only readiness summary permitted by the ExecPlan, with business logic in a service rather than the FastAPI route, or prove an existing endpoint supplies the same deterministic data.
- Readiness must be derived from actual local candle availability, at minimum symbols with candles, supported timeframes present, row count, first/latest available timestamp, and an honest ready/empty/partial state.
- Do not add PRO-03 provenance/import manifests/catalog persistence.
- Remove invented `HOSE/HNX` or `Equity` fallback facts. Unknown data must display as unknown/unavailable.

Required tests:

- symbol metadata with zero candles is not ready;
- empty DB is actionable empty;
- deterministic seeded candles are ready with exact counts/range;
- partial query failure is visibly classified and retryable.

## R02-03 — Complete Replay context and Vietnamese semantics

Current defects: Replay still renders raw date and default-locale OHLCV; timeframe, adjustment, mode/intent, timezone, and readiness/freshness are incomplete.

Required correction:

- Replay header visibly shows symbol, timeframe, adjustment, mode/intent, bar index, `dd/MM/yyyy` date, OHLCV/volume using shared `vi-VN` formatters, and explicit `Asia/Ho_Chi_Minh` semantics where needed.
- Show honest readiness/latest-data context from the narrow readiness contract without implying PRO-03 freshness/provenance guarantees.
- Keep backend/Replay controller authoritative and page/component boundaries intact.

Required tests cover zero/negative/null formatting, timezone boundary, exact context labels, and no raw default-locale fallback in the scoped surfaces.

## R02-04 — Replace weak PRO-02 UAT with acceptance-level browser evidence

The current five assertions may remain as IDs only if strengthened; add new stable blocking IDs where separate contracts need separate evidence. Never rename or weaken an accepted ID.

The deterministic browser workflow must:

1. Create or identify at least two real replay sessions, not a backtest-only session.
2. Advance the primary session and record its authoritative current index.
3. Open Dashboard with a populated Recent Practice Sessions section and click the exact Continue action.
4. Prove Replay actually loads that exact session in DOM, store-visible product state, and session API—not merely the URL.
5. Use the searchable picker to select the second real replay session and prove URL/store/product content all change together.
6. Return to the primary session and execute Dashboard → Replay → Journal → Analytics → Replay with the same exact session and unchanged authoritative current index.
7. Reload and exercise browser back/forward, proving session/index preservation.
8. Exercise invalid/deleted session URL and prove an actionable picker/clear error state rather than fallback ID `1`. Remove the UAT fallback `|| '1'`.
9. Prove Journal and Analytics require no raw numeric Session ID input.
10. Prove Vietnamese Replay context including timeframe, adjustment, mode/intent, date, OHLCV, volume, and readiness labels.
11. Prove keyboard isolation by comparing authoritative replay index/drawing state before and after typing navigation/drawing shortcut keys in the picker—not only by checking input text.
12. Exercise Dashboard ready, empty, and partial/error action contracts deterministically where practical; retain focused component coverage for states that require controlled routing.

## R02-05 — Produce required visual evidence

Required final screenshots:

- `pro02-dashboard-1440x1000.png` after animations/load settle, with readable content and a populated recent-session Continue action.
- `pro02-cross-route-workflow-1280x800.png` at exactly 1280×800, showing selected session context and usable navigation/picker without clipped core actions.

Visually inspect both. Record dimensions and SHA-256. The previous dark mid-animation Dashboard and 1440×1000 Analytics dropdown image are retained as rework history, not final acceptance evidence.

## R02-06 — Seal all accepted PRO-01 UAT assertions

Current validator seals the 265 V3 baseline and exact PRO-00 set, but accepted `pro01.*` IDs can still be removed or renamed.

Required correction:

- Add an immutable exact PRO-01 count/hash seal without weakening V3 or PRO-00 seals.
- Add independent negative tests proving removal, rename, and blocking downgrade of a PRO-01 assertion fail closed.
- PRO-02 assertions remain additive and blocking; reconciliation must reject missing/unexpected/duplicate/blocking mismatch.

## R02-07 — Close the internal `backtest` mode input boundary

Current defect: adding `SessionMode.BACKTEST` to the enum used by both response and create schema allows clients to create replay sessions with internal mode `backtest`.

Required correction:

- Preserve read/response compatibility for existing internal backtest sessions.
- Use a separate create-mode type or explicit typed validation so `POST /api/replay/sessions` rejects `mode: "backtest"` with 422.
- Keep ordinary replay modes compatible and keep backtest sessions excluded from the replay picker/list.
- Add focused create/response/list regression tests.

## R02-08 — Run the exact missing gates and finalize evidence

The prior transcript invoked nonexistent `scripts/verify-product.ps1`; therefore the full product gate is pending.

After focused corrections are green, run in order:

```powershell
node --test scripts/product-uat-manifest.test.mjs
powershell -ExecutionPolicy Bypass -File scripts/verify-v2.ps1
powershell -ExecutionPolicy Bypass -File scripts/run-product-uat.ps1
& 'C:\Program Files\Git\bin\bash.exe' -lc "cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-product.sh"
```

Also run the focused frontend/backend regressions discovered above. Commands must be bounded. Do not repeat a long failure without a concrete retained diagnosis and focused correction.

Final evidence must record:

- exact test/gate commands, exit codes, counts, and artifact path;
- manifest path/hash, declared/actual/passed/failed/blocking/missing/unexpected/duplicate/mismatch counts;
- runtime/page/console/provider/request/API outcomes;
- both final screenshot paths, dimensions, hashes, and visual-review result;
- temporary DB identity and absence after cleanup;
- production DB before/after/current hashes (exact match required);
- zero owned listeners/processes on cleanup;
- complete changed/untracked inventory, deviations, rollback, known limitations, and Reviewer checklist;
- explicit confirmation no V3/PRO-00/PRO-01 assertion was removed, renamed, weakened, duplicated, or made non-blocking.

Update `docs/exec-plans/PRO_02_DAILY_TRADER_WORKFLOW.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md` with the rework history and final evidence. Return to the Independent Reviewer gate and stop. Do not commit, push, or start PRO-03.
