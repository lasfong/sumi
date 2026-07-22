# Sumi Project Review Report — 2026-07-15

## 1. Executive summary

**Final verdict: technically works, but the chart/indicator UX is unusable for serious technical-analysis practice; the product is a prototype and the drawing subsystem needs replacement.**

Sumi does use `lightweight-charts@5.2.0`, and its current price, volume, RSI, MACD, and CCI panes are created with the official v5 single-chart pane API. The backend indicator unification is also real: replay calls `IndicatorEngine`, while backtest and scanner call `StrategyIndicatorAdapter -> IndicatorEngine`; shared rule evaluation and scoped backtest cleanup are implemented and covered by tests.

Those facts do not make the replay workspace a serious trading-analysis product. The browser UAT showed that EMA, RSI, MACD, and CCI render after sufficient warmup, but:

- there is no visible active-indicator manager;
- individual indicators cannot be removed, hidden, or configured;
- panes have no titles or legends, RSI/CCI reference lines are absent, and right-edge labels overlap;
- three oscillator panes were only 111–112 px high at a 1440×1000 viewport;
- only Cursor, Trendline, Horizontal Line, Fibonacci, and Remove All are exposed;
- drawings cannot be selected, moved, edited, styled, individually deleted, or undone;
- ray, rectangle, and text/note are not implemented;
- live multi-point drawing attempts produced `Maximum call stack size exceeded` and duplicate-time Lightweight Charts assertions; only a horizontal line reliably persisted in the final UAT;
- the existing smoke gate adds four indicators but does not verify that they are visible, manageable, correctly labeled, configurable, or usable, and it does not exercise drawing tools.

The three named community projects were **referenced only**. None is installed in `frontend/package.json` or `frontend/package-lock.json`, and none is imported by source code.

Recommended direction: keep Lightweight Charts v5 and the backend indicator engine; rebuild the Indicator Manager; replace the custom drawing implementation behind a provider adapter after a time-boxed comparison spike. Do not rewrite the entire backend or switch the base chart library yet.

## 2. Branch, tag, and provenance

| Fact | Evidence | Conclusion |
| --- | --- | --- |
| Current branch | `master` | Review is not on the RC2 tag or a named hardening branch. |
| HEAD | `108aa5d merge: post-rc2 hardening` | Post-RC2 work is present through a merge commit. |
| RC2 tag target | `812675ce37d30ddfafc11c6eeca299b5cd8a3c9e` | `v2.0.0-rc2` remains at the hardening release commit. |
| Tags at HEAD | none | Current HEAD is not tagged as RC2. |
| Working tree before review artifacts | clean (`git status --porcelain` returned no output) | Product code was reviewed without pre-existing uncommitted changes. |
| Named hardening branch | no local/remote match from `git branch --all --list '*post-rc2-hardening*'` | The branch name is absent, but its changes are merged. |

`v2.0.0-rc2..HEAD` contains `66a9ac2 test: harden post-rc2 release coverage` and merge commit `108aa5d`. The diff is 12 files, principally shared strategy evaluation, backtest cleanup, indicator parity tests, scanner/backtest changes, release evidence, and browser-smoke hardening. The chart module itself is already in the tagged `812675c` release commit; the post-tag diff did not rebuild its UX.

## 3. Dependency and library audit

| Project/package | Installed? | Used by source? | Actual status |
| --- | ---: | ---: | --- |
| `lightweight-charts@5.2.0` | Yes | Yes | Real production dependency; imported by replay, chart managers, drawing registry, and analytics chart. |
| `deepentropy/lightweight-charts-indicators` / `lightweight-charts-indicators` | No | No | Reference only in docs. Sumi computes indicators on the backend instead. |
| `deepentropy/lightweight-charts-drawing` / `lightweight-charts-drawing` | No | No | Reference only in docs. |
| `difurious/lightweight-charts-line-tools-core` | No | No | Reference only in docs. |

Evidence:

- `frontend/package.json` declares `"lightweight-charts": "^5.2.0"` and no community chart plugins.
- `frontend/package-lock.json` resolves `lightweight-charts-5.2.0.tgz` and has no entries for the named community packages.
- `npm ls lightweight-charts` resolves exactly `5.2.0`; all three plugin `npm ls` checks return `(empty)`.
- Source imports are only from `lightweight-charts`; there are no `deepentropy`, `difurious`, `DrawingManager`, or line-tools plugin imports.
- `docs/CHART_V5_TECHNICAL_PLAN.md` explicitly labels the community projects as “Reference only” or future providers.

The custom drawing code is `DrawingToolRegistry.ts` plus click/crosshair subscriptions in `CandleChart.tsx`. The custom indicator UI is `IndicatorSelector.tsx` plus state/effects in `ReplayPage.tsx`. Neither is a wrapper around a third-party drawing or indicator manager.

For current upstream context, `deepentropy/lightweight-charts-drawing` advertises v5 primitives, selection, drag editing, manager events, JSON export/import, and 68 tools under MIT; `difurious/lightweight-charts-line-tools-core` is a v5+ MPL-2.0 orchestrator whose tools are separate companion packages. These are upstream claims, not capabilities present in Sumi. See [deepentropy drawing](https://github.com/deepentropy/lightweight-charts-drawing) and [difurious line-tools core](https://github.com/difurious/lightweight-charts-line-tools-core).

## 4. Chart architecture audit

### Architecture classification: C — prototype with serious gaps

The file split is meaningful but too thin to be called an extensible chart platform.

#### What is real and structurally sound

- `CandleChart.tsx` creates one official v5 chart and delegates base series, panes, indicator series, and persisted drawing rendering to separate classes.
- `PaneManager` calls `chart.addPane(true)`, stores `IPaneApi` objects, uses `paneIndex()`, `setStretchFactor()`, and `chart.removePane()`.
- `SeriesManager` calls v5 `chart.addSeries(SeriesDefinition, options, paneIndex)` for price, volume, lines, and histograms.
- A single chart owns all panes, so the panes share one official time scale; there is no fragile multi-chart time-scale synchronization path in this module.
- Indicator warmup/null values are filtered in `IndicatorRenderRegistry`: `null`, empty, non-finite values are omitted rather than sent to the chart.
- Indicator series are keyed by indicator ID plus JSON parameters. Identical configurations are de-duplicated; the same type with different parameters can coexist in the same pane.

#### Where the split is superficial or incomplete

- There is no separate `ChartWorkspace.tsx`; `CandleChart.tsx` exports `ChartWorkspace` and aliases it as `CandleChart`.
- `CandleChart.tsx` is not a clean facade: it owns chart construction, drawing input state, OHLC snapping, preview lifecycle, subscriptions, and drawing creation.
- `ReplayPage.tsx` is 595 lines and owns session orchestration, websocket updates, candle transforms, markers, indicator persistence, sequential indicator fetching, drawing persistence, playback, trade actions, and the complete workspace layout.
- The imperative `removeIndicator(key)` API exists, but the UI never calls it. This is an implemented internal method without a usable product path.
- Adding each new indicator causes the effect to refetch every active indicator sequentially. The UAT recorded repeated EMA/RSI/MACD requests as the active array grew.
- Pane IDs are only `indicator:${indicatorId}`. Different parameterizations of the same oscillator share a pane, which is technically valid but has no manager/legend to distinguish them.
- No price-scale policy is defined for price, RSI, MACD, CCI, or volume beyond Lightweight Charts autoscaling. RSI is not fixed to 0–100 and has no 30/50/70 levels; CCI has no -100/0/100 levels.
- Pane defaults are mechanically assigned stretch factors. With price, volume, RSI, MACD, and CCI visible, the UAT measured: price 397 px, volume 87 px, RSI 111 px, MACD 111 px, CCI 112 px. This is technically rendered but cramped.
- Series titles appear only as last-value labels on the right axis. There is no pane title/legend; MACD’s three labels visibly stack over one another.

### Detailed answers

| Question | Finding |
| --- | --- |
| Correct official v5 pane API? | Yes. `addPane`, `IPaneApi`, pane indices, and v5 `addSeries` signatures are used. |
| Time scales synchronized? | Yes by construction: one chart owns all panes. |
| Pane heights usable? | Not with several indicators; 87–112 px subpanes are too cramped and unlabeled. |
| Price/volume scales sensible? | Basic autoscale works; volume uses volume formatting. No explicit policy or user control. |
| RSI scale sensible? | No. Autoscale only; no fixed 0–100 scale or 30/50/70 references. |
| MACD scale sensible? | Partially. Line, signal, and histogram render around zero, but no zero reference line and labels overlap. |
| CCI scale sensible? | Partially. Autoscale renders data, but -100/0/100 references are absent. |
| Warmup/null handling? | Yes in the render registry; null/non-finite points are omitted. |
| Multiple same-type indicators? | Backend/series keys support different parameter sets; identical settings are suppressed. UX cannot distinguish or manage them. |
| Remove individual indicator? | No user path. Only Remove All. |
| Change parameters? | Not implemented in UI. Defaults are silently applied. |
| Active indicators clear? | No. Once the add menu closes, no active list is visible. |

## 5. Indicator UX audit

### Indicator UX verdict: unusable for serious TA practice

The add action is barely usable; the resulting workspace is not manageable.

| UAT action | Result | What the user sees / practical judgement |
| --- | --- | --- |
| Create replay and advance to 61 bars | PASS | Session and bar number are visible. Warmup is sufficient for all four tested indicators. |
| Add EMA | PASS, technical | EMA line and right-axis value label render. No active list or period control. |
| Add RSI | PASS, technical | Separate pane renders. No RSI title, 30/50/70 lines, hide/remove/settings controls. |
| Add MACD | PASS, technical | Line, signal, and histogram render. Three right-axis labels overlap; no pane legend or zero line. |
| Add CCI | PASS, technical | Separate pane renders. No -100/0/100 references or pane title. |
| Volume | PASS, automatic | Volume is always rendered in its own pane; it is not an addable/toggleable indicator. |
| Identify active indicators | FAIL | No active names remain visible after closing the menu. Only chart-side series labels give indirect evidence. |
| Remove RSI | FAIL | No individual remove action. |
| Remove MACD | FAIL | No individual remove action. |
| Change EMA period | NOT IMPLEMENTED | Registry metadata includes parameter definitions, but the UI creates a default config immediately. |
| Change RSI period | NOT IMPLEMENTED | Same limitation. |
| Reload active indicators | PASS, technical | Four session-indicator requests were observed after reload and panes returned. Persistence is localStorage keyed by session. |
| Remove All, reload | PASS | No session-indicator requests occurred after reload. |
| Pane usability after reload | PARTIAL | State restores, but the same cramped/unlabeled panes return. |
| Console/runtime errors | FAIL | UAT captured `Maximum call stack size exceeded` and `data must be asc ordered by time` assertions in the combined chart/drawing path. |

Persistence caveat: `WorkspacePersistence` serializes drawings and indicators together, but `ReplayPage.loadIndicator()` and `handleClearIndicators()` save `{ drawings: [], indicators: ... }`. Backend drawing persistence is separate, so this does not erase the backend state directly, but it makes the local workspace schema misleading and risks losing local drawing state if it becomes authoritative later.

Evidence: [indicators-active.png](review-artifacts/2026-07-15/indicators-active.png) and [uat-results.json](review-artifacts/2026-07-15/uat-results.json).

## 6. Drawing tools UX audit

### Drawing tools verdict: toy-level

The system renders minimal chart series/price lines; it is not a drawing-tool interaction system.

| Tool/action | Exists? | Interactive? | Edit/move/delete? | Persists? | Serious TA suitability |
| --- | ---: | ---: | ---: | ---: | --- |
| Horizontal line | Yes | One-click placement | No / No / only Clear All | Yes in final UAT | Minimal but not manageable. |
| Trendline | Yes | Two-click code path | No selection or handles | Not reliably in UAT | Toy-level; represented as a two-point `LineSeries`. |
| Ray | No | No | No | No | Missing. |
| Rectangle | No | No | No | No | Missing. |
| Fibonacci retracement | Yes | Two-click code path | No selection, levels, labels, settings, or handles | Not reliably in UAT | Placeholder-quality. Seven finite lines plus a diagonal; no usable Fib interaction model. |
| Text/note | No | No | No | No | Missing. |
| Delete selected | No | No selection model | No | N/A | Missing. |
| Move/edit | No | No hit testing | No | N/A | Missing. |
| Undo | No | No | No | N/A | Missing. |
| Clear all | Yes | One-click trash icon | Global destructive clear only | Yes | Acceptable as one utility, insufficient alone. |
| Cancel drawing | Partial | Switch back to Cursor | No explicit Escape/cancel action | N/A | Discoverability is poor. |

The toolbar has only icons. Active state is highlighted, and hover titles identify tools, but there is no selected-drawing state, context toolbar, properties panel, keyboard delete, escape handler, undo stack, lock, visibility, or per-drawing styling.

Implementation evidence:

- Horizontal lines use `createPriceLine()` with a generic title `Drawing`.
- Trendlines use an ordinary `LineSeries` containing two points.
- Fibonacci uses seven ordinary finite `LineSeries` levels between two timestamps plus a diagonal; it has no level labels, percentages, fill zones, reversal, extension, or editing.
- `DrawingToolRegistry.render()` clears and recreates all rendered drawing references whenever the drawings array changes.
- `SumiDrawingAdapter` validates and serializes only `cursor`, `trendline`, `horizontal`, and `fibonacci`.

In the final UAT, clicking all three exposed drawing modes left one persisted horizontal line. Multi-point attempts coincided with three stack-overflow page errors and a duplicate-time chart assertion. This is evidence of unreliable interaction, not a pass merely because click handlers returned.

Evidence: [drawings.png](review-artifacts/2026-07-15/drawings.png) and [after-reload.png](review-artifacts/2026-07-15/after-reload.png).

## 7. Concrete minimum trading-analysis checklist

### Indicator Manager minimum

| Requirement | Result | Evidence |
| --- | --- | --- |
| Active indicator list visible | FAIL | No list/chips/legend outside canvas labels. |
| Add from clear menu/modal | PASS | Categorized dropdown menu works. |
| Remove with one click | FAIL | Only Remove All. |
| Toggle show/hide | NOT IMPLEMENTED | No control/state. |
| Edit parameters | NOT IMPLEMENTED | Defaults are created silently. |
| Restore after reload | PASS | Four indicator requests after reload. |
| Separate pane indicators labeled | PARTIAL | Right-axis series labels exist; panes themselves are not titled. |
| Each pane has title/legend | FAIL | None. |
| Pane resizable or reasonable default | FAIL | 87–112 px subpanes with several indicators are cramped; no visible affordance communicates resizing. |
| MACD line/signal/histogram clear | PARTIAL | All render, but labels overlap and zero reference is absent. |
| RSI 30/50/70 lines | FAIL | Absent. |
| CCI -100/0/100 lines | FAIL | Absent. |

### Drawing toolbar minimum

| Requirement | Result | Evidence |
| --- | --- | --- |
| Toolbar visible/understandable | PARTIAL | Visible icon rail; labels require hover. |
| Active tool highlighted | PASS | Blue active state. |
| Tools not confusing | PARTIAL | Four icons are simple, but Fibonacci behavior and cancellation are opaque. |
| Draw/select/move/edit/delete | FAIL | Placement exists; lifecycle after placement does not. |
| Cancel active mode | PARTIAL | Selecting Cursor cancels; no explicit Escape behavior. |
| Clear drawings | PASS | Remove All Drawings. |
| Persist after reload | PARTIAL | Horizontal line persisted; multi-point tools were unreliable in UAT. |
| Fibonacci usable | FAIL | Placeholder geometry; no labels/handles/settings; failed final UAT persistence. |
| Horizontal/Trendline/Ray/Rectangle reliable | FAIL | Horizontal worked; trendline unreliable; ray and rectangle absent. |

### Replay minimum

| Requirement | Result | Evidence |
| --- | --- | --- |
| Current candle/date obvious | PARTIAL | Bar number and OHLC are in header; current date is not explicit there. |
| Future candles hidden | PASS, architecture | Session-scoped candles/indicators use current replay state; tested indicator endpoint is session-scoped. |
| Next/previous clear | PASS | Prev/Next, ±5, keyboard arrows, autoplay. |
| Trade controls do not fight chart controls | PARTIAL | Controls are separated, but header crowding and fixed right panel materially reduce chart workspace. |
| Journal/checklist accessible | PARTIAL | Decision log is in the replay side panel; Journal is a separate sidebar route. No visible checklist in replay. |
| Practice without fighting UI | FAIL | Indicator and drawing lifecycle gaps dominate the workflow. |

## 8. Why technical gates passed while the product feels unusable

The gates test technical survival, not trading-analysis usability.

1. `scripts/browser-smoke.mjs` clicks EMA, RSI, MACD, and CCI, waits 500 ms each, then continues. It never checks pane count, labels, lines, reference levels, active state, removal, parameters, or reload.
2. The smoke creates a session and immediately adds indicators. At one initial bar, RSI/MACD/CCI may not have enough warmup data to render, yet the test still passes because it only checks that the page is nonblank.
3. The smoke never selects a drawing tool, creates a drawing, edits it, deletes it, or reloads it.
4. Successful smoke artifacts are deleted, so the green gate leaves no screenshot for human product review.
5. Unit tests cover indicator column mapping and drawing serialization validation, not canvas interaction or trader workflows.
6. “Official v5 panes render” was treated as the finish line. It should have been an infrastructure milestone followed by an explicit UX acceptance gate.

The underlying cause is a combination of vague product acceptance criteria, implementation optimized to satisfy technical checkboxes, no concrete Indicator Manager/drawing interaction spec, and no real plugin integration. The base library choice is not the main problem.

## 9. Product reality assessment

1. **Technically verified but poor UX?** Yes. More precisely: technically functional infrastructure with unusable indicator management and toy-level drawings.
2. **Current chart/drawing system salvageable?** The base chart, pane concept, backend data flow, and series abstraction are salvageable. The custom drawing implementation is not worth extending into a full interaction engine.
3. **Keep Lightweight Charts v5?** Yes. It renders the required market data efficiently, has official pane/plugin primitives, and is not the cause of the missing UX.
4. **Integrate a real drawing plugin?** Yes, after a time-boxed spike validates lifecycle, persistence mapping, performance, licensing, and React cleanup.
5. **Replace custom drawing system?** Yes. Keep a Sumi-owned provider boundary and persisted domain schema/migration layer; replace hit testing, selection, drag/edit, rendering, and tool lifecycle.
6. **Which community project to spike?** Spike `deepentropy/lightweight-charts-drawing` first because it presents a cohesive MIT manager with the required basic tools and lifecycle. Benchmark `difurious/lightweight-charts-line-tools-core` second if modular per-tool packaging is valuable; account for MPL-2.0 file-level obligations and multiple companion packages. Neither should be accepted on README claims alone.
7. **Rebuild Indicator UI?** Yes, as a real Indicator Manager. Do not adopt browser-side indicator computation merely to get a demo menu; keep the backend source of truth.
8. **Root cause?** All four named causes contributed: incomplete requirements, superficial completion criteria, absent UX specification, and deliberate non-integration of plugin systems. The docs accurately said “reference only,” but release/product language allowed that limitation to be mistaken for completion.

## 10. Keep / refactor / rebuild matrix

| Area | Current Quality | Keep | Refactor | Rebuild | Reason |
| --- | --- | ---: | ---: | ---: | --- |
| Lightweight Charts base | B | ✓ |  |  | Correct v5 dependency/API; solid base for local TA. |
| PaneManager | C |  | ✓ |  | Official API, but sizing/scale/reference-line policies and UX are too primitive. |
| SeriesManager | C+ |  | ✓ |  | Useful ownership boundary; needs richer series metadata, legends, scale options, and stable lifecycle. |
| IndicatorRenderRegistry | C+ |  | ✓ |  | Null handling and column mapping work; needs explicit render definitions per indicator, not column-prefix heuristics alone. |
| Indicator UI | D |  |  | ✓ | No active list, individual remove, toggle, settings, or pane controls. |
| DrawingToolRegistry | D |  |  | ✓ | Ordinary series/price-line renderer, no interaction lifecycle. |
| SumiDrawingAdapter | C | ✓ | ✓ | Keep the provider boundary/domain intent; add versioned migration and provider mapping. |
| Drawing tools | D |  |  | ✓ | Toy-level and missing core TA tools/actions. |
| ReplayPage layout | C |  | ✓ |  | Functional shell, but 595-line orchestration and crowded fixed regions need decomposition. |
| Trade controls | B- | ✓ | ✓ | Visible and usable in smoke; retain behavior, improve integration/layout after chart priorities. |
| Backend IndicatorEngine | B+ | ✓ | ✓ | Real registry, validation, shared computation; keep and extend tests/contracts. |
| StrategyIndicatorAdapter | B | ✓ | ✓ | Real unification path; limited parameter forwarding should become registry-driven. |
| Scanner rule evaluator | B | ✓ | ✓ | Shared safe evaluation is real and tested; keep. |
| Backtest cleanup | B | ✓ |  |  | Scoped to `mode == backtest`, dry-run support, dependent cleanup, and tests. |

## 11. Prioritized issues

### P0 — unusable/blocker

1. **No indicator lifecycle UI:** active indicators cannot be clearly identified, individually removed, hidden, or configured.
2. **Drawing lifecycle is absent:** no selection, move, edit, selected delete, undo, or reliable multi-point creation.
3. **Runtime instability in real drawing path:** stack overflows and duplicate-time chart assertions occurred during final browser UAT; only the horizontal line persisted.

### P1 — serious UX/product gap

1. RSI and CCI lack required reference lines; MACD lacks a clear zero reference and its labels overlap.
2. Oscillator panes are cramped and unlabeled; default sizing does not scale to several active studies.
3. Ray, rectangle, and text/note are missing; Fibonacci is placeholder-quality.
4. Replay owns too much chart/business/UI logic and refetches all active indicators when one is added.
5. Existing browser acceptance criteria can pass without any oscillator visibly rendering.

### P2 — improvement

1. Make indicator render definitions explicit and registry-driven, including scale, reference lines, display names, colors, and formatters.
2. Version and migrate drawing-provider persistence instead of directly coupling provider JSON to backend state.
3. Make current replay date explicit in the header and improve chart/control space allocation.
4. Test multiple configurations of the same indicator and request cancellation/race behavior.

### P3 — polish

1. Replace generic `Drawing` axis titles and raw backend column names with human labels.
2. Add accessible labels/tooltips that do not require guessing icons.
3. Improve Vietnamese-market number/date formatting consistency.

## 12. Recommended next plan

### Option A — salvage current chart, including custom drawings

Keep Lightweight Charts, PaneManager, SeriesManager, backend computation, and the custom drawing renderer. Build an Indicator Manager and implement selection/hit testing/editing in-house.

- Expected effort: **6–10 person-weeks** for a credible MVP, excluding advanced tools.
- Risks: interaction geometry, blank-space time mapping, hit testing, handles, keyboard lifecycle, persistence, and pane isolation become a custom chart-engine project. Highest regression risk and poor leverage.
- Assessment: not recommended.

### Option B — replace only the drawing subsystem, and rebuild the Indicator Manager

Keep Lightweight Charts v5, the backend engine, current session/replay APIs, and a refactored chart facade. Put a third-party drawing provider behind a Sumi adapter; preserve or migrate Sumi drawing state. In parallel, replace `IndicatorSelector` with a proper manager using backend registry metadata.

- Spike: **3–5 days** to compare deepentropy and difurious on trendline, horizontal, ray, rectangle, Fib, text, select/move/delete, export/import, replay reload, pan/zoom, and React unmount.
- Product implementation after a successful spike: **4–7 person-weeks**.
- Risks: community-project maturity, plugin API drift, provider JSON compatibility, licensing, performance with many drawings, event cleanup, and multi-pane behavior.
- Mitigation: accept only a provider that passes a Sumi-owned contract/UAT suite; keep the provider isolated.
- Assessment: **recommended**.

### Option C — rebuild the entire Replay/Chart UI

Keep the backend and either retain Lightweight Charts under a new React workspace or evaluate a different commercial/open-source chart library.

- Expected effort: **8–14+ person-weeks** before feature parity, longer if switching chart libraries.
- Risks: regression across replay, trades, markers, persistence, websocket updates, responsive layout, and licensing; likely to discard working backend/chart foundations.
- Assessment: unjustified now. Reconsider only if the drawing-provider spike fails or Lightweight Charts cannot satisfy the accepted UX contract.

### Recommended sequence

1. Freeze feature claims and write interaction-level acceptance criteria for Indicator Manager, drawings, and replay.
2. Run the drawing-provider spike on a separate branch; do not change persistence until one provider passes.
3. Rebuild the Indicator Manager against the existing backend registry and engine.
4. Refactor `ReplayPage` and chart ownership around explicit domain state and provider interfaces.
5. Add a browser UAT gate that verifies visible panes, individual remove/settings/reload, reference lines, and complete drawing lifecycle with retained screenshots.

## 13. Browser/manual UAT artifacts

- [Indicator panes after EMA/RSI/MACD/CCI](review-artifacts/2026-07-15/indicators-active.png)
- [Drawing attempt; only horizontal line visibly retained](review-artifacts/2026-07-15/drawings.png)
- [Reload state](review-artifacts/2026-07-15/after-reload.png)
- [Machine-readable UAT observations](review-artifacts/2026-07-15/uat-results.json)

The final run used Chrome headless at 1440×1000 against the local Vite/FastAPI app and seeded FPT data. Sumi’s own browser smoke also passed. The separate Chrome-control connection was unavailable in this environment, so detailed UAT used the project’s installed Playwright/Chrome stack—the same browser family and dependency used by `scripts/browser-smoke.mjs`.

## 14. Tests and exact commands run

### Provenance/dependencies/source

```bash
git status --porcelain
git branch --show-current
git log --oneline -10
git tag --points-at HEAD
git rev-list -n 1 v2.0.0-rc2
git branch --all --list '*post-rc2-hardening*'
git log --oneline v2.0.0-rc2..HEAD
git diff --stat v2.0.0-rc2..HEAD
cat frontend/package.json
rg -i -n 'lightweight' frontend/package-lock.json
cd frontend && npm ls lightweight-charts
cd frontend && npm ls lightweight-charts-indicators || true
cd frontend && npm ls lightweight-charts-drawing || true
cd frontend && npm ls lightweight-charts-line-tools-core || true
cd frontend && npm ls | rg -i chart || true
rg -n 'lightweight-charts' frontend/src
rg -n 'Drawing|TrendLine|Fibonacci|Fib|Rectangle|Ray|Horizontal|LineTool|ToolRegistry|DrawingManager' frontend/src
rg -n 'IndicatorRenderRegistry|PaneManager|SeriesManager|ChartWorkspace|SumiDrawingAdapter' frontend/src
rg -n 'deepentropy|difurious|line-tools|lightweight-charts-drawing|lightweight-charts-indicators' . -g '!frontend/node_modules/**' -g '!frontend/dist/**'
```

### Local app/UAT

```bash
cd backend && ../.venv/bin/python -m uvicorn app.main:app --host 127.0.0.1 --port 8000
cd frontend && npm run dev -- --host 127.0.0.1
curl -sS http://127.0.0.1:8000/api/health
curl -sS http://127.0.0.1:8000/api/indicators/registry
cd frontend && npm run smoke:browser
node /tmp/sumi-review-uat.mjs
curl -sS http://127.0.0.1:8000/api/replay/sessions/112/drawings
```

### Verification

```bash
cd frontend && npm test
cd frontend && npm run build
cd frontend && npm run lint
cd backend && ../.venv/bin/python -m pytest app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py app/tests/test_scanner.py app/tests/test_backtest_cleanup.py -q
cd backend && ../.venv/bin/python -m pytest -q
```

Results:

- Sumi browser smoke: PASS.
- Frontend tests: 9 files, 18 tests PASS.
- Frontend build: PASS.
- Frontend lint: PASS.
- Focused backend suite: 14 PASS.
- Full backend suite: 75 PASS, 1 skipped, 1 deprecation warning.
- Detailed product UAT: indicator rendering/persistence technically PASS; indicator management FAIL; drawing lifecycle FAIL; runtime errors observed.

## 15. Honest final verdict

**Classification: technically works but UX unusable; prototype only; keep the chart base, rebuild indicator management, and replace the drawing subsystem.**

A real trader can advance candles and see computed studies, but cannot comfortably manage those studies or perform normal technical-analysis drawing work. Calling the current replay UI “TradingView-like” would be misleading. It has a TradingView-derived rendering base and a few familiar icons, not a TradingView-like analysis workflow.
