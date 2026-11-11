"""Unit and integration tests for BatchBacktestRunner, compute-once caching, and phase isolation.

Covers:
- FR-CORE-008: Multi-symbol, multi-phase batch backtesting orchestration.
- FR-CORE-009: Independent starting capital per phase simulation (no capital bleed).
- NFR-PERF-001 / TEST-PERF-001: Compute-once indicator feature caching per symbol.
- TEST-BT-005: Phase-end position holding integrity (open positions marked to market in equity, excluded from closed trades).
- Single-symbol parity: Parity between single kernel simulation and batch slice execution.
- API endpoints: POST /api/backtest/batch/run and POST /api/backtest/run with phases.
"""

from datetime import date, timedelta
from typing import List
from unittest.mock import patch
import pytest
import pandas as pd
from fastapi.testclient import TestClient

from app.domain.backtest.batch_runner import (
    BatchBacktestRunner,
    BatchPhaseResult,
    PhaseDefinition,
    SymbolFeatureCache,
)
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.market.provider import MarketRuleProvider
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle
from app.services.backtest_service import BacktestService


def _create_synthetic_candles(symbol: str, start_date: date, num_days: int) -> pd.DataFrame:
    """Generate synthetic daily candles for testing."""
    records = []
    base_price = 50.0
    for i in range(num_days):
        day = start_date + timedelta(days=i)
        # Create a deterministic wave pattern
        price = base_price + (i % 20) * 1.5
        records.append({
            "timestamp": pd.Timestamp(day),
            "open": price - 0.5,
            "high": price + 1.0,
            "low": price - 1.0,
            "close": price,
            "volume": 500_000.0,
        })
    return pd.DataFrame(records)


def _get_test_strategy_dict() -> dict:
    """Standard strategy dictionary for testing."""
    return {
        "name": "Phase Crossover Strategy",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 5},
            {"name": "sma_slow", "type": "sma", "length": 15},
        ],
        "entry_rules": [
            {"condition": "sma_fast > sma_slow"},
        ],
        "exit_rules": [
            {"condition": "sma_fast < sma_slow"},
        ],
        "position_sizing": {
            "method": "fixed_quantity",
            "quantity": 1000,
        },
    }


# ============================================================================
# 1. Phase Definition & Spanning Window Validation
# ============================================================================

def test_phase_definition_validation():
    """Verify phase definition checks date ordering and names."""
    # Valid phase
    p = PhaseDefinition(name="Phase 1", start_date="2024-01-01", end_date="2024-03-31")
    p.validate()

    # Empty name
    with pytest.raises(ValueError, match="name cannot be empty"):
        PhaseDefinition(name="", start_date="2024-01-01", end_date="2024-03-31").validate()

    # Inverted dates
    with pytest.raises(ValueError, match="strictly before"):
        PhaseDefinition(name="Invalid", start_date="2024-04-01", end_date="2024-01-01").validate()

    # Same dates
    with pytest.raises(ValueError, match="strictly before"):
        PhaseDefinition(name="Invalid", start_date="2024-01-01", end_date="2024-01-01").validate()


def test_batch_runner_validate_phases():
    """Verify validate_phases rejects empty lists and duplicates."""
    with pytest.raises(ValueError, match="cannot be empty"):
        BatchBacktestRunner.validate_phases([])

    dup_phases = [
        PhaseDefinition(name="Bull", start_date="2024-01-01", end_date="2024-03-31"),
        PhaseDefinition(name="Bull", start_date="2024-04-01", end_date="2024-06-30"),
    ]
    with pytest.raises(ValueError, match="Duplicate phase name"):
        BatchBacktestRunner.validate_phases(dup_phases)


def test_determine_spanning_window():
    """Verify spanning window correctly incorporates earliest start + warmup and latest end."""
    phases = [
        PhaseDefinition(name="P1", start_date="2024-03-01", end_date="2024-06-01"),
        PhaseDefinition(name="P2", start_date="2024-07-01", end_date="2024-12-31"),
    ]
    warmup_start, latest_end = BatchBacktestRunner.determine_spanning_window(phases, warmup_days=90)
    # Earliest start 2024-03-01 minus 90 days is 2023-12-02
    assert warmup_start == "2023-12-02"
    assert latest_end == "2025-01-01"


# ============================================================================
# 2. Compute-Once Feature Caching (NFR-PERF-001, TEST-PERF-001)
# ============================================================================

def test_compute_once_feature_caching():
    """TEST-PERF-001 / NFR-PERF-001:
    Indicator computation must run exactly S times for S symbols, regardless of P phases.
    """
    strategy_dict = _get_test_strategy_dict()
    strategy = load_strategy_from_dict(strategy_dict)

    symbols = ["FPT", "VNM"]
    phases = [
        PhaseDefinition(name="Q1", start_date="2024-01-01", end_date="2024-03-31"),
        PhaseDefinition(name="Q2", start_date="2024-04-01", end_date="2024-06-30"),
        PhaseDefinition(name="Q3", start_date="2024-07-01", end_date="2024-09-30"),
    ]

    # Pre-generate candle cache for provider
    candles_db = {
        "FPT": _create_synthetic_candles("FPT", date(2023, 10, 1), 365),
        "VNM": _create_synthetic_candles("VNM", date(2023, 10, 1), 365),
    }

    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        return candles_db[sym]

    original_compute = StrategyIndicatorAdapter.compute
    compute_call_count = 0

    def spied_compute(*args, **kwargs):
        nonlocal compute_call_count
        compute_call_count += 1
        return original_compute(*args, **kwargs)

    with patch.object(StrategyIndicatorAdapter, "compute", side_effect=spied_compute):
        result = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=candle_provider,
            initial_cash_per_run=100_000_000.0,
        )

    # Invariant: compute called exactly len(symbols) times (2), NOT symbols * phases (6)
    assert compute_call_count == 2
    assert result.feature_compute_count == 2
    assert result.total_symbols == 2
    assert result.total_phases == 3
    assert result.total_runs == 6
    assert result.simulation_run_count == 6
    assert result.status == "succeeded"


# ============================================================================
# 3. Independent Capital Isolation (FR-CORE-009)
# ============================================================================

def test_independent_capital_isolation():
    """FR-CORE-009: Each (symbol, phase) run must start with fresh initial capital.
    Profits or losses from previous phases must never bleed into subsequent phases.
    """
    strategy_dict = _get_test_strategy_dict()
    strategy = load_strategy_from_dict(strategy_dict)

    phases = [
        PhaseDefinition(name="Phase 1", start_date="2024-01-01", end_date="2024-04-01"),
        PhaseDefinition(name="Phase 2", start_date="2024-04-01", end_date="2024-07-01"),
    ]

    initial_capital = 150_000_000.0
    candles_df = _create_synthetic_candles("FPT", date(2023, 10, 1), 300)

    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        return candles_df

    result = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=["FPT"],
        phases=phases,
        candle_provider=candle_provider,
        initial_cash_per_run=initial_capital,
    )

    assert len(result.phase_results) == 2
    res_p1 = result.phase_results[0]
    res_p2 = result.phase_results[1]

    # Both phases must report exactly the designated initial capital
    assert res_p1.initial_cash == initial_capital
    assert res_p2.initial_cash == initial_capital

    # Even if p1 ended with a different equity / cash, p2's initial_cash is strictly isolated
    assert res_p2.initial_cash != res_p1.final_cash or res_p1.final_cash == initial_capital
    assert res_p2.initial_cash == 150_000_000.0


# ============================================================================
# 4. Phase-End Holding Integrity (TEST-BT-005)
# ============================================================================

def test_phase_end_holding_integrity():
    """TEST-BT-005: Open positions at phase end must be marked to market in equity,
    but strictly excluded from closed trade metrics (total_trades counts closed only).
    """
    # Create a strategy that enters immediately but never exits
    hold_strategy_dict = {
        "name": "Always Hold Strategy",
        "indicators": [
            {"name": "sma5", "type": "sma", "length": 5},
        ],
        "entry_rules": [
            {"condition": "sma5 > 0"},  # Always true after warmup
        ],
        "exit_rules": [
            {"condition": "sma5 < 0"},  # Never true
        ],
        "position_sizing": {
            "method": "fixed_quantity",
            "quantity": 1000,
        },
    }
    strategy = load_strategy_from_dict(hold_strategy_dict)

    phases = [
        PhaseDefinition(name="Holding Phase", start_date="2024-01-01", end_date="2024-02-15"),
    ]

    candles_df = _create_synthetic_candles("SSI", date(2023, 10, 1), 150)

    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        return candles_df

    initial_cash = 100_000_000.0
    result = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=["SSI"],
        phases=phases,
        candle_provider=candle_provider,
        initial_cash_per_run=initial_cash,
    )

    assert result.status == "succeeded"
    res = result.phase_results[0]

    # 1. Closed trades count is 0 because the position was not closed before cutoff
    assert res.total_trades == 0

    # 2. Position is actively held at phase end
    assert res.open_position_quantity == 1000
    assert res.open_position_value > 0

    # 3. Final equity incorporates marked-to-market open position value
    expected_equity = res.final_cash + res.open_position_value
    assert round(res.final_equity, 2) == round(expected_equity, 2)
    assert res.net_pnl == round(res.final_equity - initial_cash, 2)


# ============================================================================
# 5. Single-Symbol Batch Parity
# ============================================================================

def test_single_symbol_batch_parity():
    """Verify that a batch run with 1 symbol and 1 phase produces identical results
    to direct execution via BacktestExecutionKernel.
    """
    strategy_dict = _get_test_strategy_dict()
    strategy = load_strategy_from_dict(strategy_dict)

    phase = PhaseDefinition(name="TestPhase", start_date="2024-01-01", end_date="2024-05-01")
    candles_df = _create_synthetic_candles("TCB", date(2023, 10, 1), 250)

    def candle_provider(sym: str, start: str, end: str) -> pd.DataFrame:
        return candles_df

    # Batch run
    batch_res = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=["TCB"],
        phases=[phase],
        candle_provider=candle_provider,
        initial_cash_per_run=100_000_000.0,
    )

    batch_run = batch_res.phase_results[0]
    assert batch_run.status == "succeeded"

    # Direct run with same sliced data
    cache = BatchBacktestRunner.build_symbol_feature_cache("TCB", candles_df, strategy)
    sliced_df, sliced_ind = cache.slice_phase(phase)

    profile = MarketRuleProvider.get_profile("vietnam_default_conservative", exchange="HOSE")
    kernel = BacktestExecutionKernel(
        fee_rate=0.0015,
        tax_rate=0.0010,
        slippage_rate=0.0,
        min_holding_bars=2,
        profile=profile,
        exchange="HOSE",
    )
    direct_res = kernel.run(
        df=sliced_df,
        indicator_values=sliced_ind,
        strategy=strategy,
        symbol="TCB",
        initial_cash=100_000_000.0,
        timeframe="1D",
        exchange="HOSE",
    )

    assert batch_run.final_cash == direct_res.final_cash
    assert batch_run.total_trades == len([t for t in direct_res.trades if t.status == "closed"])
    assert batch_run.open_position_quantity == direct_res.final_position.quantity


# ============================================================================
# 6. Service & API Endpoints
# ============================================================================

@pytest.mark.asyncio
async def test_backtest_service_run_batch(db_session):
    """Test BacktestService.run_batch_backtest with database fixture."""
    base_date = date(2024, 1, 1)
    candles = []
    for i in range(120):
        candles.append(Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=25.0 + (i % 10) * 0.5,
            high=26.0 + (i % 10) * 0.5,
            low=24.5 + (i % 10) * 0.5,
            close=25.5 + (i % 10) * 0.5,
            volume=2_000_000,
        ))
    db_session.add_all(candles)
    db_session.commit()

    service = BacktestService()
    config = {
        "symbols": ["HPG"],
        "phases": [
            {"name": "P1", "start_date": "2024-01-15", "end_date": "2024-02-28"},
            {"name": "P2", "start_date": "2024-03-01", "end_date": "2024-04-15"},
        ],
        "strategy": _get_test_strategy_dict(),
        "initial_cash": 100_000_000.0,
    }

    result = await service.run_batch_backtest(db_session, config)
    assert result["status"] == "succeeded"
    assert result["total_symbols"] == 1
    assert result["total_phases"] == 2
    assert result["total_runs"] == 2
    assert result["feature_compute_count"] == 1
    assert result["simulation_run_count"] == 2
    assert len(result["phase_results"]) == 2


def test_batch_api_endpoint(client, db_session):
    """Test POST /api/backtest/batch/run via FastAPI TestClient."""
    base_date = date(2024, 1, 1)
    candles = []
    for i in range(100):
        candles.append(Candle(
            symbol="MBB",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=20.0 + (i % 8) * 0.4,
            high=21.0 + (i % 8) * 0.4,
            low=19.5 + (i % 8) * 0.4,
            close=20.5 + (i % 8) * 0.4,
            volume=1_500_000,
        ))
    db_session.add_all(candles)
    db_session.commit()

    payload = {
        "symbols": ["MBB"],
        "phases": [
            {"name": "Phase Jan", "start_date": "2024-01-10", "end_date": "2024-02-10"},
            {"name": "Phase Feb", "start_date": "2024-02-10", "end_date": "2024-03-10"},
        ],
        "strategy": _get_test_strategy_dict(),
        "initial_cash": 100_000_000.0,
    }

    response = client.post("/api/backtest/batch/run", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "succeeded"
    assert data["total_symbols"] == 1
    assert data["total_phases"] == 2
    assert data["total_runs"] == 2
    assert data["feature_compute_count"] == 1
    assert len(data["phase_results"]) == 2


def test_run_backtest_with_phases_routing(client, db_session):
    """Test POST /api/backtest/run with phases routing transparently to batch runner."""
    base_date = date(2024, 1, 1)
    candles = []
    for i in range(100):
        candles.append(Candle(
            symbol="VRE",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=22.0,
            high=23.0,
            low=21.0,
            close=22.5,
            volume=1_000_000,
        ))
    db_session.add_all(candles)
    db_session.commit()

    payload = {
        "symbol": "VRE",
        "phases": [
            {"name": "Phase 1", "start_date": "2024-01-10", "end_date": "2024-02-10"},
        ],
        "strategy": _get_test_strategy_dict(),
        "initial_cash": 100_000_000.0,
    }

    response = client.post("/api/backtest/run", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "succeeded"
    assert data["total_symbols"] == 1
    assert data["total_phases"] == 1
    assert len(data["phase_results"]) == 1
