# Sumi v2.0.0-rc2 Release Verification Log

Date: 2026-07-04  
Reviewer mode: independent release reviewer  
Candidate workspace: `/Users/mizuhara/workspace/sumi-v2-rc2-verify-20260704`  
Source workspace: `/Users/mizuhara/workspace/sumi`

## 1. Executive Result

Disposition: **TECHNICAL GO for local-first SQLite daily-candle RC2 after remediation; release provenance still requires commit/tag/push and optional clean-tag rerun.**

Reason:

- Core install, migration, seed, backend tests, frontend lint/tests/build and browser product smoke passed.
- The previous P1 indicator architecture risk has been fixed: Backtest and Scanner now compute strategy indicators through the shared `IndicatorEngine` via `StrategyIndicatorAdapter`.
- Browser smoke now selects CCI through the Replay UI, so CCI pane behavior is covered by browser verification, not only source/unit tests.
- Strategy Lab sweep now returns compact variant payloads and no longer pushes full equity curves into UI/localStorage history.
- The remaining process risk is Git provenance: the candidate must be committed, tagged as `v2.0.0-rc2`, pushed, and ideally rerun from that tag for final release evidence.

## 2. Verification Workspace Setup

### 2.1 Source Git State

Command:

```bash
git status --short --branch
```

Output:

```text
## codex/lightweight-charts-v5-spike
 M README.md
 M backend/app/api/health.py
 M backend/app/config.py
 M backend/app/domain/accounting.py
 M backend/app/main.py
 M backend/app/services/analytics_service.py
 M backend/app/services/data_quality_service.py
 M backend/app/services/trade_lifecycle_service.py
 M backend/app/tests/test_accounting.py
 M backend/app/tests/test_api_integration.py
 M backend/app/tests/test_cafef_importer.py
 M backend/app/tests/test_scanner.py
 M backend/app/tests/test_trade_lifecycle.py
 M docs/HANDOFF_REPORT_2026-07-03.md
 M docs/INDEX.md
 M docs/PRODUCT_COMPLETION_PLAN_2026-07-04.md
 M docs/RELEASE_CHECKLIST_V2.md
 M frontend/src/App.tsx
 M frontend/src/api/client.ts
 M frontend/src/components/chart/DrawingToolbar.tsx
 M frontend/src/components/layout/Sidebar.css
 M frontend/src/components/layout/Sidebar.tsx
 M frontend/src/index.css
 M frontend/src/pages/ReplayPage.tsx
 M scripts/browser-smoke.mjs
?? backend/app/tests/test_analytics_known_ledger.py
?? docs/RELEASE_EVIDENCE_2026-07-04.md
?? scripts/verify-v2.sh
```

Finding: **PARTIAL**. A clone was clean initially, but the rc2 candidate itself is not fully committed. The verification workspace was created by:

```bash
git clone /Users/mizuhara/workspace/sumi /Users/mizuhara/workspace/sumi-v2-rc2-verify-20260704
git diff --binary > /tmp/sumi-v2-rc2-candidate.patch
git apply /tmp/sumi-v2-rc2-candidate.patch
cp untracked rc2 files into the verify workspace
```

This is acceptable for an engineering audit of the current candidate, but not acceptable as final release provenance.

### 2.2 Python Runtime

Command:

```bash
python3.12 --version
```

Output:

```text
Python 3.12.13
```

Result: **PASS**

### 2.3 Backend Dependencies From Scratch

Commands:

```bash
python3.12 -m venv .venv
.venv/bin/python -m pip install --upgrade pip
.venv/bin/python -m pip install -r backend/requirements.txt
```

Key output:

```text
Successfully installed ... fastapi-0.139.0 ... sqlalchemy-2.0.51 ...
pandas-ta-0.4.71b0 ... pytest-9.1.1 ... alembic-1.18.5 ...
```

Result: **PASS**

### 2.4 Frontend Dependencies From Scratch

Command:

```bash
cd frontend
npm install
```

Output:

```text
added 277 packages, and audited 278 packages in 2s
found 0 vulnerabilities
```

Result: **PASS**

## 3. Fresh Database Migration And Seed

### 3.1 Fresh Alembic Migration

Command:

```bash
DATABASE_URL=sqlite:////tmp/sumi-v2-rc2-verify-20260704.db ../.venv/bin/python -m alembic upgrade head
```

Output:

```text
INFO  [alembic.runtime.migration] Running upgrade  -> cdf80254e9dc, initial_schema
INFO  [alembic.runtime.migration] Running upgrade cdf80254e9dc -> 20260629_0001, strategy_lab_runs
INFO  [alembic.runtime.migration] Running upgrade 20260629_0001 -> 20260629_0002, replay_session_source
INFO  [alembic.runtime.migration] Running upgrade 20260629_0002 -> 20260629_0003, scanner_runs
INFO  [alembic.runtime.migration] Running upgrade 20260629_0003 -> 20260629_0004, execution_trade_id_drift
```

Current revision:

```text
20260629_0004 (head)
```

Result: **PASS**

### 3.2 Deterministic Demo Seed

Command:

```bash
DATABASE_URL=sqlite:////tmp/sumi-v2-rc2-verify-20260704.db ../.venv/bin/python scripts/seed_demo.py
```

Output:

```text
Seeded 2080 deterministic daily candles (FPT=520, SSI=520, VCI=520, VNINDEX=520).
```

Result: **PASS**

## 4. Automated Gate Results

### 4.0 Latest Remediation Gate

After fixing indicator engine consistency, Strategy Lab sweep payload size and
CCI browser coverage, the latest gate was:

```bash
./scripts/verify-v2.sh
```

Output summary:

```text
Backend tests: 72 passed, 1 skipped, 1 warning
Fresh database migration: pass
Frontend lint: pass
Frontend tests: 9 files / 18 tests passed
Frontend production build: pass
Browser smoke skipped by script flag
Sumi V2 verification passed.
```

Browser smoke was run separately against seeded local services:

```bash
SUMI_BROWSER_CHANNEL=chrome npm run smoke:browser
```

Output:

```text
Sumi browser smoke passed
```

### 4.1 Backend Pytest

Command:

```bash
cd backend
../.venv/bin/python -m pytest app/tests -q
```

Output:

```text
......................................................................   [100%]
70 passed, 1 warning in 0.79s
```

Warning:

```text
StarletteDeprecationWarning: Using `httpx` with `starlette.testclient` is deprecated; install `httpx2` instead.
```

Result: **PASS with accepted warning candidate**

### 4.2 Frontend Lint

Command:

```bash
cd frontend
npm run lint
```

Output:

```text
> frontend@0.0.0 lint
> eslint .
```

Result: **PASS**

### 4.3 Frontend Tests

Command:

```bash
cd frontend
npm test -- --run
```

Output:

```text
Test Files  9 passed (9)
Tests       18 passed (18)
```

Runtime note:

```text
ExperimentalWarning: localStorage is not available because --localstorage-file was not provided.
```

Result: **PASS with accepted test-runtime warning candidate**

### 4.4 Frontend Production Build

Command:

```bash
cd frontend
npm run build
```

Output:

```text
vite v8.0.16 building client environment for production...
✓ 1901 modules transformed.
✓ built in 630ms
```

Result: **PASS**

### 4.5 Browser Product Smoke

Services:

```bash
DATABASE_URL=sqlite:////tmp/sumi-v2-rc2-verify-20260704.db ../.venv/bin/python -m uvicorn app.main:app --host 127.0.0.1 --port 8000
npm run dev -- --host 127.0.0.1 --port 5173
```

First smoke attempt:

```text
[TypeError: fetch failed]
cause: Error: connect ECONNREFUSED 127.0.0.1:8000
```

Root cause: backend bind inside sandbox failed:

```text
ERROR: [Errno 1] error while attempting to bind on address ('127.0.0.1', 8000): [errno 1] operation not permitted
```

Retest after running backend with approved local bind:

```bash
SUMI_BROWSER_CHANNEL=chrome npm run smoke:browser
```

Output:

```text
Sumi browser smoke passed
```

Smoke coverage observed in `scripts/browser-smoke.mjs`:

- Creates replay session for `FPT`.
- Adds EMA, RSI and MACD through UI.
- Executes BUY.
- Verifies T+2 rejection on early SELL.
- Advances replay and closes position.
- Runs Backtest with `macd_rsi_momentum.yaml`.
- Runs Strategy Lab comparison and sweep.
- Runs Scanner and opens Replay from a scanner signal.
- Loads Analytics for the smoke session.
- Checks mobile routes for blank page and horizontal overflow.
- Fails on page errors or unexpected console errors.

Result: **PASS after environment bind issue was resolved**

## 5. Lightweight Charts v5 Verification

### 5.1 Installed Version

Command:

```bash
cd frontend
npm ls lightweight-charts
```

Output:

```text
frontend@0.0.0 /Users/mizuhara/workspace/sumi-v2-rc2-verify-20260704/frontend
└── lightweight-charts@5.2.0
```

Result: **PASS**

### 5.2 v5 Pane API Usage

Source evidence:

- `frontend/src/components/chart/PaneManager.ts`
  - `chart.panes()[0]`
  - `chart.addPane(true)`
  - `pane.paneIndex()`
  - `chart.removePane(pane.paneIndex())`
- `frontend/src/components/chart/SeriesManager.ts`
  - `chart.addSeries(CandlestickSeries, options, panes.index('price'))`
  - `chart.addSeries(HistogramSeries, options, panes.index('volume'))`
  - `chart.addSeries(LineSeries, options, paneIndex)`
- `frontend/src/components/chart/DrawingToolRegistry.ts`
  - drawing series are added to the price pane via `chart.addSeries(LineSeries, options, paneIndex)`.

Search for old v4-style API:

```bash
rg -n "addCandlestickSeries|addLineSeries|addHistogramSeries|setMarkers\\(" frontend/src -S
```

Output:

```text
frontend/src/components/chart/SeriesManager.ts:39:    this.markerPlugin.setMarkers(markers);
```

Finding:

- No `addCandlestickSeries`, `addLineSeries` or `addHistogramSeries` remains in `frontend/src`.
- `setMarkers()` remains only on the v5 marker plugin returned by `createSeriesMarkers`, not on a v4 series API.

Result: **PASS**

## 6. Indicator Pane And Warmup Verification

### 6.1 RSI/MACD/CCI Pane Mapping

Source evidence:

```typescript
IndicatorRenderRegistry.paneFor('ema', 'main') -> 'price'
IndicatorRenderRegistry.paneFor('rsi', 'oscillator') -> 'indicator:rsi'
IndicatorRenderRegistry.paneFor('cci', 'oscillator') -> 'indicator:cci'
IndicatorRenderRegistry.paneFor('macd', 'oscillator') -> 'indicator:macd'
```

Test evidence:

```text
frontend/src/components/chart/__tests__/IndicatorRenderRegistry.test.ts
it('assigns overlays to price and oscillators to dedicated panes', ...)
```

Browser evidence:

- Browser smoke added EMA, RSI, MACD and CCI from the actual Replay UI.

Result:

- EMA overlay: **PASS**
- RSI separate pane: **PASS**
- MACD separate pane: **PASS**
- CCI separate pane: **PASS**

### 6.2 MACD Shape

Test evidence:

```text
IndicatorRenderRegistry maps:
MACD_12_26_9 -> line
MACDs_12_26_9 -> line
MACDh_12_26_9 -> histogram
```

Result: **PASS**

### 6.3 Null/Warmup Values

Source evidence:

```typescript
if (rawValue === null || rawValue === '') return [];
const value = typeof rawValue === 'number' ? rawValue : Number(rawValue);
return Number.isFinite(value) ? [{ time, value }] : [];
```

Test evidence:

```text
it('does not turn backend warm-up nulls into zero values', ...)
```

Expected fixture:

```text
[{ timestamp: '2026-01-01', EMA_20: null },
 { timestamp: '2026-01-02', EMA_20: 101.5 }]
```

Rendered output:

```text
[{ time: '2026-01-02', value: 101.5 }]
```

Result: **PASS**

## 7. Trade Lifecycle Verification

Targeted command:

```bash
../.venv/bin/python -m pytest \
  app/tests/test_trade_lifecycle.py::test_buy_rejects_when_cash_is_insufficient \
  app/tests/test_trade_lifecycle.py::test_close_without_open_position_is_rejected \
  app/tests/test_trade_lifecycle.py::test_sell_t_plus_1_is_rejected \
  app/tests/test_accounting.py::test_known_ledger_cash_reconciles_across_partial_and_full_close \
  -q
```

Output:

```text
....                                                                     [100%]
4 passed, 1 warning in 0.06s
```

Coverage:

- Insufficient cash BUY rejected: **PASS**
- CLOSE without open position rejected: **PASS**
- SELL before T+2 rejected: **PASS**

## 8. Accounting Fixture Verification

Market rule constants:

```python
BUY_FEE_RATE = 0.0015
SELL_FEE_RATE = 0.0015
SELL_TAX_RATE = 0.001
```

Fixture:

- Initial cash: `100_000.0`
- Buy 100 @ 100:
  - Gross: `10_000.0`
  - Buy fee: `15.0`
  - Cash out: `10_015.0`
- Buy 100 @ 110:
  - Gross: `11_000.0`
  - Buy fee: `16.5`
  - Cash out: `11_016.5`
- Cash after two buys: `78_968.5`
- Partial sell 50 @ 120:
  - Gross: `6_000.0`
  - Sell fee: `9.0`
  - Sell tax: `6.0`
  - Cash in: `5_985.0`
  - Cash after partial reduce: `84_953.5`
- Final sell 150 @ 90:
  - Gross: `13_500.0`
  - Sell fee: `20.25`
  - Sell tax: `13.5`
  - Cash in: `13_466.25`
  - Cash after full close: `98_419.75`
- Total buy cash out: `21_031.5`
- Total sell cash in: `19_451.25`
- Net PnL: `-1_580.25`
- PnL percent: `-7.51372940589%`

Result: **PASS**

## 9. Indicator Engine Consistency Verification

### 9.1 BacktestService

Finding: **FIXED**

`BacktestService` no longer owns `_compute_indicators()`. It now calls:

```python
StrategyIndicatorAdapter.compute(df, strategy.indicators)
```

`StrategyIndicatorAdapter` maps strategy indicator names such as `macd_line`,
`macd_signal`, `macd_hist` and `rsi` from shared `IndicatorEngine` output.

Regression evidence:

```text
test_strategy_indicator_adapter_uses_shared_indicator_engine_outputs
```

This test compares Backtest strategy indicator values against direct
`IndicatorEngine.compute()` output.

### 9.2 ScannerService

Finding: **FIXED**

`ScannerService` now uses the same adapter:

```python
StrategyIndicatorAdapter.compute(df, strategy.indicators)
```

Scanner, Backtest and Replay now share the same indicator source of truth.

## 10. Risk Table

| Status | Severity | Area | Finding | Required Action Before GO |
|---|---:|---|---|---|
| FIXED | P0 | Install/bootstrap | Backend and frontend dependencies install from scratch. | None. |
| FIXED | P0 | Database | Fresh SQLite Alembic migration reaches `20260629_0004 (head)`. | None. |
| FIXED | P0 | Demo data | Deterministic seed creates 2,080 candles. | None. |
| FIXED | P0 | Automated gates | Backend pytest, frontend lint, frontend tests and production build pass. | None. |
| FIXED | P0 | Browser UAT | Browser product smoke passes after local backend bind permission is granted. | None. |
| FIXED | P1 | Chart dependency | `lightweight-charts@5.2.0` installed. No v4 add-series API remains in `frontend/src`. | None. |
| FIXED | P1 | Indicator warmup | Frontend render registry skips null/empty/non-finite indicator values instead of rendering zero. | None. |
| FIXED | P1 | Trading lifecycle | Insufficient cash BUY, CLOSE without open position and SELL before T+2 are rejected. | None. |
| FIXED | P1 | Accounting | Buy fee, sell fee, sell tax, partial cash, full-close cash and net PnL reconcile with hand-calculated fixture. | None. |
| FIXED | P1 | Indicator architecture | Backtest and Scanner now use `StrategyIndicatorAdapter` over shared `IndicatorEngine`. | None. |
| FIXED | P1 | Strategy Lab UX/performance | Sweep variants are compact and history persistence cannot crash UI on localStorage quota. | None. |
| PARTIAL | P1 | Release provenance | rc2 candidate is not a pure committed clean checkout; verification required applying uncommitted patch and untracked files. | Commit rc2 candidate, tag `v2.0.0-rc2`, rerun verification from that tag. |
| FIXED | P2 | Browser pane evidence | Browser smoke selects EMA, RSI, MACD and CCI through the Replay UI. | None. |
| ACCEPTED LIMITATION candidate | P2 | Test runtime | Frontend test emits Node localStorage experimental warning. | Accept or configure Vitest/Node localStorage file. |
| ACCEPTED LIMITATION candidate | P2 | Backend test runtime | Starlette/httpx deprecation warning appears. | Accept for rc2 or pin/upgrade test stack later. |

## 11. Final Reviewer Decision

I declare **technical GO for RC2 local-first/internal review scope**, with one remaining release-process condition: commit/tag/push and optionally rerun from the tag.

P0 status:

- No open P0 found in this verification run.

P1 status:

- No open P1 product/architecture issue remains in this verification run.
- Release provenance is **PARTIAL** until the rc2 candidate is committed/tagged/pushed.

GO condition:

1. Commit and tag the rc2 candidate.
2. Push branch and `v2.0.0-rc2` tag.
3. Rerun this verification pack from the committed tag if formal release provenance is required.
