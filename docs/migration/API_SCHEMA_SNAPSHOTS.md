# Sumi V3 API Schema Snapshots & Frozen Contracts

**Document Version**: 3.0.0  
**Date**: 2026-09-19  
**Status**: Frozen Production Schemas  

This document freezes the JSON and Pydantic schemas for the APIs delivered across the Sumi DEV Program (P1–P10) for reference by downstream clients and API consumers.

---

## 1. Signal Registry & Calculation API (`/api/signals`)

### 1.1 `GET /api/signals/registry`
Returns the complete dictionary of 72 registered signal definitions.

**Response Schema (`SignalRegistryResponse`)**:
```json
{
  "signals": [
    {
      "name": "volume.relative_volume",
      "version": "1.0.0",
      "category": "volume",
      "label_vi": "Khối lượng tương đối (RVOL)",
      "description": "Tỷ lệ giữa khối lượng nến hiện tại so với trung bình N nến trước đó.",
      "output_type": "float",
      "parameters_schema": {
        "period": { "type": "int", "default": 20, "minimum": 1, "maximum": 252 }
      },
      "default_parameters": { "period": 20 },
      "dependencies": [],
      "warmup_bars": 20,
      "causal_delay_bars": 0,
      "status": "ACTIVE",
      "ast_alias": "volume__relative_volume"
    }
  ]
}
```

### 1.2 `POST /api/signals/replay/{session_id}/calculate`
Calculates up to 2 signals over the authoritative replay session prefix ending at `current_index`.

**Request Schema (`SignalCalculationRequest`)**:
```json
{
  "signals": [
    {
      "name": "health.score",
      "version": "1.0.0",
      "params": { "preset": "health_v1_balanced" }
    },
    {
      "name": "bb.direction_rising",
      "params": { "horizon": "T20" }
    }
  ]
}
```

**Response Schema (`SignalCalculationResponse`)**:
```json
{
  "session_id": 1,
  "observed_index": 50,
  "candle_count": 51,
  "timeframe": "1D",
  "series": [
    {
      "signal_name": "health.score",
      "signal_version": "1.0.0",
      "resolved_params": { "preset": "health_v1_balanced" },
      "params_hash": "a4f9255e...",
      "points": [
        {
          "bar_index": 50,
          "timestamp": "2026-01-15T00:00:00",
          "output_type": "float",
          "value": 45.5,
          "quality": "VALID",
          "reasons": ["EMA20 > EMA50", "RSI neutral positive"],
          "threshold": null
        }
      ]
    }
  ]
}
```

---

## 2. Multi-Phase Batch Backtest API (`/api/backtest/batch/run`)

### 2.1 Request Schema (`BatchBacktestRequest`)
```json
{
  "symbols": ["FPT", "SSI", "HPG"],
  "phases": [
    {
      "name": "In-Sample (2020-2022)",
      "start_date": "2020-01-01",
      "end_date": "2022-12-31",
      "description": "Bull market and correction"
    },
    {
      "name": "Out-of-Sample (2023-2024)",
      "start_date": "2023-01-01",
      "end_date": "2024-12-31",
      "description": "Divergent recovery"
    }
  ],
  "strategy": {
    "name": "EMA_Cross_Research",
    "version": "1.0.0",
    "indicators": [
      { "name": "ema20", "type": "ema", "length": 20 },
      { "name": "ema50", "type": "ema", "length": 50 }
    ],
    "entry_rules": [{ "condition": "ema20 > ema50" }],
    "exit_rules": [{ "condition": "ema20 < ema50" }],
    "position_sizing": { "method": "fixed_quantity", "quantity": 1000 },
    "risk_management": { "stop_loss_pct": 0.07, "take_profit_pct": 0.15 }
  },
  "initial_cash": 100000000,
  "benchmark_symbol": "VNINDEX",
  "execution_profile": "vietnam_default_conservative",
  "exchange": "HOSE",
  "use_cache": true
}
```

### 2.2 Response Schema (`BatchBacktestResponse`)
```json
{
  "status": "succeeded",
  "total_symbols": 3,
  "total_phases": 2,
  "total_runs": 6,
  "feature_compute_count": 3,
  "simulation_run_count": 6,
  "timing_metrics": {
    "total_duration_ms": 145.2,
    "feature_compute_ms": 12.8,
    "simulation_ms": 128.4,
    "cache_hits": 3,
    "cache_misses": 0
  },
  "phase_results": [
    {
      "symbol": "FPT",
      "phase_name": "In-Sample (2020-2022)",
      "start_date": "2020-01-01",
      "end_date": "2022-12-31",
      "status": "completed",
      "total_candles": 740,
      "initial_cash": 100000000.0,
      "final_cash": 125400000.0,
      "final_equity": 125400000.0,
      "net_pnl": 25400000.0,
      "net_return_pct": 25.4,
      "total_trades": 18,
      "open_position_quantity": 0,
      "open_position_value": 0.0,
      "warnings": [],
      "benchmark_metrics": {
        "ticker": "FPT",
        "initial_cash": 100000000.0,
        "final_cash": 125400000.0,
        "final_equity": 125400000.0,
        "net_profit": 25400000.0,
        "net_profit_pct": 25.4,
        "num_trades": 18,
        "win_rate_pct": 61.11,
        "profit_factor": 2.14,
        "max_drawdown": 12.4
      }
    }
  ],
  "metric_matrix": {
    "phase_names": ["In-Sample (2020-2022)", "Out-of-Sample (2023-2024)"],
    "symbols": ["FPT", "SSI", "HPG"],
    "consistency_score": 78.5,
    "cross_phase_degradations": []
  },
  "markdown_table": "| Ticker | Phase | Return % | Win Rate % | Profit Factor | Max DD % | Trades |\n|---|---|---|---|---|---|---|...",
  "csv_export": "symbol,phase,net_return_pct,win_rate_pct,profit_factor,max_drawdown,total_trades\n..."
}
```

---

## 3. Money Flow Bollinger Bands API (`/api/bb`)

### 3.1 `GET /api/bb/symbol/{symbol}`
Returns multi-horizon BB flow metrics for a single symbol.

**Query Parameters**:
- `timeframe`: string (default `"1D"`)
- `start_date`: string ISO date
- `end_date`: string ISO date

**Response Schema (`SymbolMoneyFlowBBResponse`)**:
```json
{
  "symbol": "FPT",
  "methodology": "OHLCV_PROXY",
  "horizons": {
    "T03": { "bandwidth": 0.045, "pct_b": 0.72, "quality": "HIGH" },
    "T05": { "bandwidth": 0.062, "pct_b": 0.68, "quality": "HIGH" },
    "T10": { "bandwidth": 0.091, "pct_b": 0.61, "quality": "HIGH" },
    "T20": { "bandwidth": 0.124, "pct_b": 0.58, "quality": "HIGH" },
    "T50": { "bandwidth": 0.185, "pct_b": 0.52, "quality": "HIGH" },
    "T200": { "bandwidth": 0.312, "pct_b": 0.49, "quality": "HIGH" }
  },
  "audit_disclaimer": "OHLCV_PROXY flow metrics are empirical estimations derived from price-volume behavior and do not replace exchange-matched order book trade aggressor classification."
}
```

### 3.2 `GET /api/bb/market/breadth`
Returns aggregated market-wide Money Flow BB breadth across `SUMI-420`.

**Response Schema (`MarketMoneyFlowBBResponse`)**:
```json
{
  "date": "2026-09-18",
  "universe_id": "SUMI-420",
  "total_securities": 420,
  "eligible_securities": 418,
  "methodology": "OHLCV_PROXY",
  "breadth_ratio": 0.64,
  "breadth_regime": "EXPANSION_POSITIVE",
  "horizons_confluence_pct": 58.2
}
```

---

## 4. Universe API (`/api/universe`)

### 4.1 `GET /api/universe/list`
Lists configured universes and member counts.

**Response Schema**:
```json
[
  {
    "universe_id": "VN30",
    "name": "VN30 Index Universe",
    "description": "30 largest market-cap stocks on HOSE",
    "active_member_count": 30,
    "last_updated": "2026-09-11"
  },
  {
    "universe_id": "HOSE50",
    "name": "HOSE 50 Top Liquidity",
    "description": "50 highest average volume stocks on HOSE",
    "active_member_count": 50,
    "last_updated": "2026-09-11"
  },
  {
    "universe_id": "SUMI-420",
    "name": "SUMI-420 Canonical Vietnam Equities Universe",
    "description": "Top 420 liquid securities meeting continuous trading and disclosure criteria",
    "active_member_count": 420,
    "last_updated": "2026-09-11"
  }
]
```
