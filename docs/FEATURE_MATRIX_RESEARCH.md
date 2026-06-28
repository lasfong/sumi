# Sumi Feature Matrix Research

Status: Product research input for Sumi V2 specs.
Date: 2026-06-28.

This document separates sourced facts from Sumi product inferences. Links point to the product/vendor documentation or official product pages used during this review.

## Sources Reviewed

| Platform | Source |
| --- | --- |
| TradingView Bar Replay | https://www.tradingview.com/support/solutions/43000474024-how-do-i-turn-bar-replay-on/ |
| TradingView Pine Script strategies | https://www.tradingview.com/pine-script-docs/concepts/strategies/ |
| AmiBroker portfolio backtester | https://www.amibroker.com/guide/h_portfolio.html |
| MetaStock product / PowerTools | https://www.metastock.com/ |
| TrendSpider product | https://trendspider.com/ |
| Portfolio123 product/docs | https://www.portfolio123.com/ and https://www.portfolio123.com/doc/doc_detail.jsp?factor=Simulation |
| QuantConnect backtesting results | https://www.quantconnect.com/docs/v2/cloud-platform/backtesting/results |
| QuantConnect LEAN | https://www.quantconnect.com/lean |
| Backtrader documentation | https://www.backtrader.com/docu/quickstart/quickstart/ |
| Backtesting.py documentation | https://kernc.github.io/backtesting.py/ |
| vectorbt documentation | https://vectorbt.dev/ and https://vectorbt.dev/api/portfolio/base/ |

## Feature Comparison

| Capability | Professional reference pattern | Sumi implication |
| --- | --- | --- |
| Bar replay | TradingView centers the workflow around selecting a replay point and revealing future bars step by step. | Manual replay must be the main product surface, not a secondary demo chart. Backend must enforce visible-only candles and indicators. |
| Indicator workspace | TradingView/Pine, AmiBroker AFL and MetaStock all treat indicators/signals as first-class reusable logic. | Build an indicator registry with parameter schemas, output series metadata and no-future-leak computation paths. Do not hard-code indicators only inside React. |
| Strategy language | Pine Script, AFL and QuantConnect LEAN all provide a disciplined strategy contract, not arbitrary expressions. | Sumi must remove `eval()` from strategy rules. MVP should use a safe declarative rule DSL before any user Python execution. |
| Portfolio backtesting | AmiBroker/Portfolio123/QuantConnect emphasize portfolio-level results, constraints and rankings, not only single-symbol PnL. | Sumi Professional v1 should support multi-symbol runs and grouped output by symbol, setup, signal, regime and period. |
| Scanner / ranking | TrendSpider and Portfolio123 compete on scanning, watchlists, ranking and finding setups. | Scanner is not MVP, but signal taxonomy and result tables must be designed now so scanner can reuse them later. |
| Backtest analytics | QuantConnect, Backtesting.py, vectorbt and AmiBroker report statistics, equity, drawdown and trade lists. | Automated backtest outputs must include trade distribution, outlier impact, profit factor, expectancy, max drawdown and benchmark comparison. |
| Research speed | vectorbt is designed for vectorized portfolio simulations and parameter sweeps. | Future Sumi Strategy Lab can add parameter grids after the event-driven broker/accounting logic is correct. |
| Realistic execution | Backtrader/QuantConnect model broker, orders and portfolio state. | Sumi should share one broker/accounting engine between manual replay and automated backtest to avoid divergent PnL rules. |

## Key Product Insights

Facts from sources:

- TradingView Bar Replay is a training UX, not merely a chart rendering option. The user selects a historical point and proceeds bar by bar.
- Pine Script strategies use explicit strategy commands and a broker-emulator model rather than free-form UI expressions.
- AmiBroker supports portfolio-level backtesting and ranking/position sizing concepts.
- QuantConnect's backtest result surface includes statistics, charts, orders, insights/logs and related result artifacts.
- Backtrader and Backtesting.py both use a strategy class pattern with lifecycle methods.
- vectorbt focuses on fast array/vectorized research, portfolios from signals and rich statistics.

Sumi inferences:

- Sumi should position itself as a local-first learning lab first, systematic research lab second.
- Manual replay should own the emotional/user-skill loop: chart reading, decision capture, execution simulation, review.
- Automated backtest should answer method-quality questions: where does a setup work, under what regime, on which symbols, and whether results depend on outliers.
- A code editor is lower priority than a safe rule system, a correct broker model and result slicing.
- Scanner, regime classifier and ranking should be designed as shared research primitives, but implemented after core correctness gates pass.

## Anti-Patterns To Avoid

- Do not implement a "Backtest" button that returns only total PnL.
- Do not let frontend receive future candles or future indicator values and hide them client-side.
- Do not run arbitrary expressions from API input with Python `eval()`.
- Do not maintain separate manual PnL logic and automated PnL logic.
- Do not add Celery/Redis, cloud execution or complex optimization before local sync runs are deterministic and tested.
- Do not expand indicator count before indicator architecture and no-future-leak tests are locked.

