# Sumi Professional v3.0.0 Release Notes

**Release Version**: `v3.0.0`  
**Release Date**: 2026-09-12  
**Quality Verification**: 193 pytest backend tests, 193 vitest frontend tests, 31 browser E2E UAT scenarios — all passed. 0 lint errors, 0 build warnings.  
**Database SHA-256 Invariant**: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (100% intact).

---

## Executive Overview

Sumi Professional v3.0.0-rc1 is a local-first manual replay, backtesting, technical analysis, data management, trading practice, journaling, and strategy research workstation engineered for Vietnam equities (HOSE, HNX, UPCOM). 

Unlike public web charting sites, Sumi operates 100% offline with zero telemetry, zero cloud tracking, and complete privacy. All market data, trading journals, strategy parameter sweeps, and replay sessions remain strictly on your local computer.

---

## Key Milestone Capabilities (PRO-00 through PRO-12)

### 1. Honest Signal Review & Fail-Closed Replay (`PRO-00`, `PRO-INT-01..10`)
- **Strict `current_index` Boundary**: Replay APIs slice data strictly at the current candle index, eliminating future-candle or future-indicator leaks.
- **Blind Practice vs Signal Review**: Start replay in blind mode or jump directly to a signal candle with boundary reveal controls.
- **Replay State Restoration**: Save and restore replay sessions seamlessly.

### 2. Backtest Trust & Honest Metrics (`PRO-01`, `PRO-BT-01..10`)
- **Metric Trust Contracts**: Refuse false precision and report zero-division, short-sample, or low-trade warnings.
- **Fee, Tax & Slippage Accounting**: Accurate Vietnam brokerage fee, tax, and slippage calculations.
- **SQN & Robustness Gates**: System Quality Number (SQN) gates requiring minimum 30 trades for statistical confidence.

### 3. Unified Daily Workflow (`PRO-UX-01..09`, `PRO-02`)
- **Searchable Session Picker**: Global workflow navigation connecting Replay, Trading Lab, Journal, and Analytics into one daily routine.
- **Context Preservation**: Seamless switching between pages without losing active session state.
- **Vietnamese Localization**: Full Vietnamese number formatting, date formats, and financial terminology.

### 4. Offline Data Catalog & Quality Import (`PRO-03`, `PRO-DATA-01..07`)
- **CafeF EOD & Custom CSV Import**: Fast local file ingestion with automated column mapping (`Date`, `Open`, `High`, `Low`, `Close`, `Volume`).
- **Conflict & Duplicate Classification**: Pre-commit preview classifying candles into New, Duplicate, Conflict, or Invalid.
- **Authoritative Weekly Aggregation**: Internal `WeeklyAggregator` deriving 1W candles using standard Monday-Friday Vietnam trading weeks (`VN_TRADING_WEEK_V1`).

### 5. Technical Indicators Suite (`PRO-04`, `PRO-05`, `PRO-06`, `PRO-07`, `PRO-IND-01..06`)
- **Overlap Indicators**: Simple Moving Average (SMA), Exponential Moving Average (EMA), Bollinger Bands, Keltner Channels, Parabolic SAR (PSAR), SuperTrend.
- **Oscillators & Momentum**: Relative Strength Index (RSI), MACD, Commodity Channel Index (CCI), Average True Range (ATR), Volume SMA, Money Flow Index (MFI), Stochastic Oscillator, Average Directional Index (ADX).
- **Benchmark Relative Strength**: Relative Strength vs `VNINDEX` or `VN30` for market regime context.
- **Ichimoku Cloud**: Complete Tenkan-sen, Kijun-sen, Senkou Span A/B, and Chikou Span cloud calculations with strict no-look-ahead displacement.

### 6. Interactive Chart Drawings (`PRO-DRAW-01..06`)
- **Drawing Tools**: Trendline, Horizontal Line, Rectangle, Fibonacci Retracement, Risk-Reward Tool.
- **Persistent Geometry**: Save, edit, drag, select, and inspect drawing properties across sessions.

### 7. Risk-Based Trade Planning & Rich Journaling (`PRO-08`, `PRO-TRADE-01..10`)
- **Position Sizing Calculator**: Risk-based position size calculation adhering to 100-share Vietnam lot increments and available cash constraints.
- **Trade Planning Checklist**: Mandatory pre-trade checklist snapshot persisted with trade execution records.
- **T+2 Settlement Enforcement**: Vietnam stock settlement rules ($T+2$) enforced during practice trading.
- **Exporting**: Local CSV and JSON journal exports.

### 8. Strategy Research & Overfitting Protection (`PRO-09`, `PRO-STRAT-01..07`)
- **Typed Parameter Discovery**: Dynamic parameter inspection for Python trading strategies.
- **AST-Safe Execution**: Declarative strategy evaluation without dangerous `eval()`.
- **In-Sample vs Out-of-Sample (OOS)**: Non-overlapping date range splitting to test strategy robustness against curve-fitting.
- **Multi-Metric Robustness Scoring**: Automated classification (`Robust`, `Overfitted`, `Low Sample`, `Unvalidated`, `Unprofitable`, `Degraded`).

### 9. Market Data Provider Boundary & One-Click Sync (`PRO-10`, `PRO-11`, `PRO-PROV-01..06`, `PRO-DATA-08..10`)
- **Provider Boundary Adapter**: Abstract interface (`MarketDataProviderAdapter`) isolating third-party network APIs from core domain logic (ADR-002).
- **Supported Providers**: SSI FastConnect (official broker API) and `vnstock` community fallback adapter.
- **One-Click Online Data Sync**: User-triggered sync with dry-run conflict preview, progress bar, atomic DB commit, automatic weekly aggregation, immutable audit manifests (`SUMI_SYNC_MANIFEST_V1`), and safe transactional rollback.

### 10. Professional Release Hardening (`PRO-12`, `PRO-REL-01..06`)
- **Performance**: High-capacity query performance (< 200ms latency on 2,000+ daily bars).
- **Backup & Restore**: Tested file backup/restore integrity with 100% SHA-256 payload matching.
- **Documentation**: Comprehensive guides for keyboard navigation, platform privacy, backup/recovery, and acceptance limits in `docs/release/`.

---

## Installation & Running Locally

### Prerequisites
- Python 3.12 or 3.13
- Node.js 22 or 24

### Backend Setup
```powershell
Set-Location backend
.\.venv\Scripts\python.exe -m pip install -r requirements.txt
.\.venv\Scripts\python.exe -m alembic upgrade head
.\.venv\Scripts\python.exe -m uvicorn app.main:app --port 8000
```

### Frontend Setup
```powershell
Set-Location frontend
npm install
npm run dev
```

Open browser at `http://localhost:5173`.

---

## Acceptance Status Matrix

| Category | Requirement IDs | Scope & Capability | Status |
| --- | --- | --- | --- |
| **Global Quality** | `PRO-G-01..10` | Full backend/frontend test suite, Alembic migrations, ESLint, Vite build, UAT manifest fail-closed authority, temporary DB hash isolation, local privacy, ExecPlan & Reviewer verification. | **PASSED** |
| **Integrity & Replay** | `PRO-INT-01..10` | Strict `current_index` cutoff preventing future candle/indicator leaks; blind practice vs signal review intent; boundary signal reveal; persistent state restoration. | **PASSED** |
| **Backtest Trust** | `PRO-BT-01..10` | Honest coverage metrics, fee/tax/slippage accounting, SQN min-30 trade gate, Sharpe/Sortino downside sample checks, reproducible backtest manifests. | **PASSED** |
| **Daily Workflow** | `PRO-UX-01..09` | Searchable session picker across Replay/Journal/Analytics, preserve workspace context, Vietnamese date/number formatting, 1440×1000 & 1280×800 layout. | **PASSED** |
| **Data Catalog & Import** | `PRO-DATA-01..07` | Catalog inspectability, pre-commit import preview with classification (new, duplicate, conflict, invalid), weekly aggregation, atomic commit. | **PASSED** |
| **Data Provider Sync** | `PRO-DATA-08..10`, `PRO-PROV-01..06` | SSI FastConnect & `vnstock` provider adapters under Provider Boundary Adapter pattern, dry-run diff preview, audit manifest, atomic rollback. | **PASSED** |
| **Core Technical Indicators** | `PRO-IND-01..06` | SMA, EMA, RSI, MACD, Bollinger Bands, ATR, Volume SMA, MFI, Stochastic, ADX, Relative Strength vs VNINDEX, Keltner Channels, PSAR, SuperTrend, Ichimoku Cloud. | **PASSED** |
| **Chart Drawings** | `PRO-DRAW-01..06` | Trendline, Horizontal Line, Rectangle, Fib Retracement, Risk-Reward tool; hit testing, selection handles, inspector properties, persistent drawing storage. | **PASSED** |
| **Trade Planning & Sizing** | `PRO-TRADE-01..10` | Standard 100-share Vietnam lot increments, risk-based position sizing, fee/tax modeling, T+2 settlement tracking, immutable checklist snapshot, local CSV/JSON exports. | **PASSED** |
| **Strategy Research** | `PRO-STRAT-01..07` | Typed parameter discovery, AST-safe declarative strategy validation, non-overlapping In-Sample vs Out-of-Sample evaluation, overfitting classification, sweep manager. | **PASSED** |

## System Limits & Operational Notes

- **Minimum Display Resolution**: 1280×800. Optimal experience at 1440×1000 or higher.
- **Lot Increments**: Vietnam HOSE/HNX standard 100-share lot minimum. Fractional share trading is excluded by domain rules.
- **T+2 Settlement Rules**: Shares bought on day T become available for sale on day T+2 trading day boundary.
- **Market Data Feeds**: Offline CafeF CSV import baseline plus online SSI FastConnect / `vnstock` provider adapters. Market sync is user-triggered on-demand only (no daemon background network polling).
- **SQLite Performance**: Verified up to 10,000+ daily bars per symbol with calculation latency under 200ms.

