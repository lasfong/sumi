import sys
from pathlib import Path
from types import SimpleNamespace
import pandas as pd
import pytest

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from datetime import datetime, timezone
from app.services.strategy_lab_service import StrategyLabService
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle


def _make_dummy_trade(net_pnl: float, status: str = "closed") -> SimpleNamespace:
    return SimpleNamespace(net_pnl=net_pnl, status=status)


def _setup_service():
    svc = StrategyLabService()
    strategy_obj = load_strategy_from_dict({
        "name": "TestStrategy",
        "indicators": [],
        "entry_rules": [],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    df = pd.DataFrame([
        {"timestamp": datetime(2024, 1, 2, tzinfo=timezone.utc), "open": 100.0, "high": 105.0, "low": 98.0, "close": 102.0, "volume": 1000},
        {"timestamp": datetime(2024, 1, 3, tzinfo=timezone.utc), "open": 102.0, "high": 108.0, "low": 101.0, "close": 106.0, "volume": 1500},
    ])
    candles = [
        Candle(symbol="FPT", timeframe="1D", adjustment_type="unadjusted", timestamp=datetime(2024, 1, 2, tzinfo=timezone.utc), open=100.0, high=105.0, low=98.0, close=102.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", adjustment_type="unadjusted", timestamp=datetime(2024, 1, 3, tzinfo=timezone.utc), open=102.0, high=108.0, low=101.0, close=106.0, volume=1500),
    ]
    return svc, strategy_obj, df, candles


def test_strategy_lab_win_rate_standardized_to_fraction():
    """SQA-013: metric_results['win_rate']['value'] must be a 0..1 fraction (e.g. 0.50, 0.25)

    consistent with summary['win_rate'] and analytics['win_rate'], preventing double-scaling
    in the frontend formatPercent (which multiplies by 100).
    """
    svc, strategy_obj, df, candles = _setup_service()

    # 4 closed trades: 2 wins (+1M, +2M) and 2 losses (-500k, -500k) -> 50% win rate
    trades = [
        _make_dummy_trade(1_000_000.0),
        _make_dummy_trade(2_000_000.0),
        _make_dummy_trade(-500_000.0),
        _make_dummy_trade(-500_000.0),
    ]
    kernel_res = SimpleNamespace(trades=trades)

    resp = svc._format_kernel_response(
        kernel_res=kernel_res,
        strategy_obj=strategy_obj,
        symbol="FPT",
        df=df,
        candles=candles,
        initial_cash=100_000_000.0,
        benchmark_symbol="VNINDEX",
    )

    # 1. Authoritative unit contract: 0..1 fraction
    win_rate_metric = resp["analytics"]["metrics"]["win_rate"]
    assert win_rate_metric["value"] == 0.50, f"Expected 0.50 fraction, got {win_rate_metric['value']}"
    assert win_rate_metric["status"] == "valid"
    assert win_rate_metric["sample_size"] == 4

    # 2. Consistency across response sections
    assert resp["summary"]["win_rate"] == 0.50
    assert resp["analytics"]["win_rate"] == 0.50

    # 3. Frontend formatPercent contract verification: formatPercent(val) = `${(val * 100).toFixed(2)}%`
    def frontend_format_percent(val):
        return f"{(val * 100):.2f}%" if val is not None else "0.00%"

    rendered = frontend_format_percent(win_rate_metric["value"])
    assert rendered == "50.00%", f"Expected '50.00%', got '{rendered}'"
    assert "5000" not in rendered, "Double-scaling detected!"


def test_strategy_lab_win_rate_edge_cases():
    """Verify 0%, 25%, 100%, and zero-trade (null / not_applicable) states."""
    svc, strategy_obj, df, candles = _setup_service()

    def frontend_format_percent(val):
        return f"{(val * 100):.2f}%" if val is not None else "0.00%"

    # Case A: 0% win rate (0 wins out of 4)
    trades_0 = [_make_dummy_trade(-100_000.0) for _ in range(4)]
    resp_0 = svc._format_kernel_response(
        SimpleNamespace(trades=trades_0),
        strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
    )
    metric_0 = resp_0["analytics"]["metrics"]["win_rate"]
    assert metric_0["value"] == 0.0
    assert frontend_format_percent(metric_0["value"]) == "0.00%"

    # Case B: 25% win rate (1 win out of 4)
    trades_25 = [_make_dummy_trade(500_000.0)] + [_make_dummy_trade(-100_000.0) for _ in range(3)]
    resp_25 = svc._format_kernel_response(
        SimpleNamespace(trades=trades_25),
        strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
    )
    metric_25 = resp_25["analytics"]["metrics"]["win_rate"]
    assert metric_25["value"] == 0.25
    assert frontend_format_percent(metric_25["value"]) == "25.00%"

    # Case C: 100% win rate (4 wins out of 4)
    trades_100 = [_make_dummy_trade(500_000.0) for _ in range(4)]
    resp_100 = svc._format_kernel_response(
        SimpleNamespace(trades=trades_100),
        strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
    )
    metric_100 = resp_100["analytics"]["metrics"]["win_rate"]
    assert metric_100["value"] == 1.0
    assert frontend_format_percent(metric_100["value"]) == "100.00%"

    # Case D: No closed trades (sample size 0)
    resp_empty = svc._format_kernel_response(
        SimpleNamespace(trades=[]),
        strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
    )
    metric_empty = resp_empty["analytics"]["metrics"]["win_rate"]
    assert metric_empty["value"] is None
    assert metric_empty["status"] == "not_applicable"
    assert metric_empty["sample_size"] == 0


def test_strategy_lab_nearby_metrics_no_double_scaling():
    """Verify that total_net_pnl, profit_factor, and expectancy maintain correct unscaled values."""
    svc, strategy_obj, df, candles = _setup_service()

    # 4 trades: 2 wins (+2M, +1M), 2 losses (-500k, -500k)
    # total_net_pnl = 2M
    # gross_profit = 3M, gross_loss = 1M -> profit_factor = 3.0
    # expectancy = 2M / 4 = 500,000.0
    trades = [
        _make_dummy_trade(2_000_000.0),
        _make_dummy_trade(1_000_000.0),
        _make_dummy_trade(-500_000.0),
        _make_dummy_trade(-500_000.0),
    ]
    resp = svc._format_kernel_response(
        SimpleNamespace(trades=trades),
        strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
    )
    metrics = resp["analytics"]["metrics"]

    assert metrics["total_net_pnl"]["value"] == 2_000_000.0
    assert metrics["profit_factor"]["value"] == 3.0
    assert metrics["expectancy"]["value"] == 500_000.0
