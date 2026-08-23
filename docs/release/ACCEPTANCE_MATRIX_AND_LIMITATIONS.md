# Sumi Professional Release Matrix & Operational Notes

## 1. Aggregate Acceptance Matrix

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

## 2. System Limits & Operational Notes

- **Minimum Display Resolution**: 1280×800. Optimal experience at 1440×1000 or higher.
- **Lot Increments**: Vietnam HOSE/HNX standard 100-share lot minimum. Fractional share trading is excluded by domain rules.
- **T+2 Settlement Rules**: Shares bought on day $T$ become available for sale on day $T+2$ trading day boundary.
- **Market Data Feeds**: Offline CafeF CSV import baseline plus online SSI FastConnect / `vnstock` provider adapters. Market sync is user-triggered on-demand only (no daemon background network polling).
- **SQLite Performance**: Verified up to 10,000+ daily bars per symbol with calculation latency under 200ms.
