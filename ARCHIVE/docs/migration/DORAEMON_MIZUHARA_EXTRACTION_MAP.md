# Doraemon & Mizuhara Subsystem Extraction Map

**Target Consumers**: Doraemon (Data Platform / Analytics) & Mizuhara (Automated Execution / Alpha Engines)  
**Sumi Release**: v3.0.0  
**Date**: 2026-09-19  

---

## 1. Overview & Extraction Principles

All Sumi domain logic is designed under Clean Architecture principles:
- **Zero Framework Coupling**: Domain models, signals, backtest logic, and market rules have zero dependencies on FastAPI, SQLAlchemy, or browser libraries.
- **Pure Functional Core**: Calculations accept canonical dataclasses, primitive types, or standard `pandas.DataFrame` / `numpy` arrays.
- **Deterministic Parity**: All modules adhere to `TEST-REPRO-001` reproducibility standards.

---

## 2. Subsystem Extraction Catalog

### Subsystem A: Next-Event Backtest Kernel
- **Location in Sumi**: `backend/app/domain/backtest/`
- **Core Files**:
  - `kernel.py`: `BacktestExecutionKernel` (state machine, order ledger, fill simulator, cash/equity tracking).
  - `models.py`: `BacktestOrder`, `BacktestTrade`, `BacktestPosition`, `BacktestState`.
  - `batch_runner.py`: `BatchBacktestRunner` (multi-symbol, multi-phase runner with feature caching and independent capital).
  - `benchmark_metrics.py`: Complete 9-metric computation (Net Return %, Win Rate, Profit Factor, Max Drawdown %, etc.).
- **External Dependencies**: `pandas`, `numpy`, `pydantic`.
- **Adoption Target (Mizuhara)**:
  - Can be packaged as `mizuhara-backtest-kernel` Python wheel.
  - Used for portfolio simulation, parameter sweeps, and walk-forward matrix evaluation.

### Subsystem B: Vietnam Market Rule Profiles
- **Location in Sumi**: `backend/app/domain/market/`
- **Core Files**:
  - `rules.py`: `VietnamMarketRules` (effective-dated price steps, price limits ±7% / ±10% / ±15%, 100 lot sizing).
  - `settlement.py`: `VietnamSettlementCalendar` (T+2 / T+2.5 trading day settlement accounting).
- **External Dependencies**: Standard library `datetime`, `math`.
- **Adoption Target (Mizuhara / Doraemon)**:
  - Can be packaged as `vn-market-rules`.
  - Authoritative rule validation for order preparation and position accounting.

### Subsystem C: Signal Registry & Causal Signals (72 Registered Signals)
- **Location in Sumi**: `backend/app/domain/signals/`
- **Core Files**:
  - `registry.py`: `SignalRegistry` (authoritative definitions, parameter schema, validation, dispatch).
  - `models.py`: `SignalDefinition`, `SignalOutputPoint`, `SignalQuality`, `SignalOutputType`, `compute_canonical_params_hash`.
  - `patterns.py`: 14 Candlestick pattern classifiers.
  - `regimes.py`: Market regime detectors (Uptrend, Downtrend, Sideways, Pullback, Recovery).
  - `vsa.py`: Volume Spread Analysis (Spring, Upthrust, Strong Demand, etc.).
  - `technical.py`: EMA crosses, RSI level crosses, MACD signal crosses.
  - `ichimoku.py`: Causal Ichimoku signals (Tenkan/Kijun cross, Kumo breakout).
  - `divergence.py`: Confirmed pivot high/low detection and RSI/MACD/Stoch divergences.
  - `health.py`: Composite Technical Health score ([-100, +100]).
  - `flow_events.py`: Money Flow BB direction, regime, confluence, and turn events.
- **External Dependencies**: `pandas`, `numpy`.
- **Adoption Target (Doraemon & Mizuhara)**:
  - Can be packaged as `sumi-signals-core`.
  - Used by Doraemon for historical signal screening and batch scanning.
  - Used by Mizuhara for strategy signal triggers and multi-factor models.

### Subsystem D: Money Flow Bollinger Bands Engine (`OHLCV_PROXY`)
- **Location in Sumi**: `backend/app/domain/bb/`
- **Core Files**:
  - `calculator.py`: `MoneyFlowBBCalculator` (multi-horizon T03–T200, bandwidth, %b, quality categorization).
  - `models.py`: `MoneyFlowBBPoint`, `MoneyFlowBBSeries`, `BBQualityStatus`.
  - `market_bb.py`: `MarketMoneyFlowBBAggregator` (market-wide breadth aggregator across SUMI-420).
- **External Dependencies**: `numpy`, `pandas`.
- **Adoption Target (Doraemon)**:
  - Can be scheduled as daily EOD pipeline to generate symbol and market breadth flow metrics.

### Subsystem E: Universe & Governance Framework
- **Location in Sumi**: `backend/app/domain/universe/`
- **Core Files**:
  - `models.py`: `UniverseDefinition`, `UniverseMembershipRule`, `UniverseMember`.
  - `backend/scripts/generate_sumi420_candidate.py`: Selection algorithm for SUMI-420 candidate universe.
- **External Dependencies**: `pydantic`.
- **Adoption Target (Doraemon)**:
  - Authoritative definition of tradeable Vietnam equity universes (`VN30`, `HOSE50`, `SUMI-420`).

### Subsystem F: Bounded Invalidation-Safe Domain Cache
- **Location in Sumi**: `backend/app/domain/engine/cache.py`
- **Core Files**:
  - `cache.py`: `DomainCache`, `compute_candle_signature`, `compute_strategy_indicators_signature`.
- **External Dependencies**: Standard library `collections.OrderedDict`, `threading`, `hashlib`.
- **Adoption Target (Doraemon / Mizuhara)**:
  - High-performance, zero-external-dependency cache for repeated strategy sweeps and feature computation.

---

## 3. Integration Recipes

### Example: Running Signal Evaluation in Downstream Python Pipeline
```python
from app.domain.signals.models import CandleBar
from app.domain.signals.registry import SignalRegistry

# Convert raw data into canonical CandleBar series
candles = [
    CandleBar(index=i, timestamp="2026-01-01", open=100.0, high=105.0, low=99.0, close=103.0, volume=500000.0)
    for i in range(100)
]

# Calculate causal Technical Health Score
result = SignalRegistry.calculate(
    name="health.score",
    candles=candles,
    params={"preset": "health_v1_balanced"},
)

for pt in result.points[-5:]:
    print(f"Bar {pt.bar_index}: Health Score = {pt.value} (Quality: {pt.quality})")
```

### Example: Executing a Multi-Phase Batch Backtest in Mizuhara
```python
from app.domain.backtest.batch_runner import BatchBacktestRunner, PhaseDefinition
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.engine.cache import DomainCache

cache = DomainCache(max_entries=500, enabled=True)
phases = [
    PhaseDefinition(name="InSample", start_date="2024-01-01", end_date="2024-12-31"),
    PhaseDefinition(name="OutSample", start_date="2025-01-01", end_date="2025-08-01"),
]

batch_res = BatchBacktestRunner.run_batch(
    strategy=my_strategy,
    symbols=["FPT", "SSI", "HPG", "TCB"],
    phases=phases,
    candle_provider=my_data_provider,
    cache=cache,
    use_cache=True,
)

print(f"Total Net PnL: {batch_res.summary['total_net_pnl']:,} VND")
print(f"Consistency Score: {batch_res.metric_matrix.consistency_score:.1f}/100")
```
