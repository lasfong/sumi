import sys
from pathlib import Path
from datetime import datetime, timezone, timedelta
import numpy as np
import pandas as pd
import pytest

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.services.backtest_service import BacktestService
from app.models.candle import Candle
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from app.db import Base
import app.models  # ensure models are registered on Base.metadata


def _make_candles(count: int, symbol: str = "FPT") -> list[Candle]:
    base = datetime(2024, 1, 2, tzinfo=timezone.utc)
    candles = []
    price = 25000.0
    for i in range(count):
        candles.append(
            Candle(
                symbol=symbol,
                timeframe="1D",
                adjustment_type="unadjusted",
                timestamp=base + timedelta(days=i),
                open=price,
                high=price + 200.0,
                low=price - 200.0,
                close=price + 50.0,
                volume=1_000_000,
            )
        )
        price += 100.0
    return candles


def test_sqa_014_strategy_indicator_adapter_short_history_populates_nan():
    """SQA-014: Short history produces NaN series instead of missing keys in indicator_values."""
    strategy_config = {
        "name": "EMA Crossover",
        "version": "1.0",
        "description": "Fast and slow EMA",
        "indicators": [
            {"name": "ema_fast", "type": "ema", "length": 20},
            {"name": "ema_slow", "type": "ema", "length": 50},
        ],
        "entry_rules": [{"condition": "ema_fast > ema_slow and close > ema_fast"}],
        "exit_rules": [{"condition": "close < ema_slow"}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    strategy = load_strategy_from_dict(strategy_config)

    # 10 candles is shorter than EMA 20 and EMA 50
    candles = _make_candles(10)
    df = pd.DataFrame([{
        "timestamp": c.timestamp,
        "open": c.open,
        "high": c.high,
        "low": c.low,
        "close": c.close,
        "volume": c.volume,
    } for c in candles])

    indicator_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)

    # Both indicator names must be present in output dictionary
    assert "ema_fast" in indicator_values
    assert "ema_slow" in indicator_values

    # Output values should be NaN arrays of length 10
    assert len(indicator_values["ema_fast"]) == 10
    assert np.isnan(indicator_values["ema_fast"]).all()
    assert len(indicator_values["ema_slow"]) == 10
    assert np.isnan(indicator_values["ema_slow"]).all()

    # StrategyRuleEvaluator.validate_strategy_rules must succeed with no 'Unknown identifier' error
    StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))


def test_sqa_014_backtest_service_returns_insufficient_warmup_diagnostic():
    """SQA-014: BacktestService returns clean INSUFFICIENT_WARMUP_HISTORY diagnostic on short history."""
    engine = create_engine("sqlite:///:memory:")
    Base.metadata.create_all(engine)
    Session = sessionmaker(bind=engine)
    db = Session()

    try:
        candles = _make_candles(10, symbol="FPT")
        for c in candles:
            db.add(c)
        db.commit()

        svc = BacktestService()
        strategy_config = {
            "name": "EMA Crossover",
            "version": "1.0",
            "indicators": [
                {"name": "ema_fast", "type": "ema", "length": 20},
                {"name": "ema_slow", "type": "ema", "length": 50},
            ],
            "entry_rules": [{"condition": "ema_fast > ema_slow"}],
            "exit_rules": [{"condition": "ema_fast < ema_slow"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        }

        res = pytest.run_async(svc.run_backtest(
            db=db,
            config={
                "strategy": strategy_config,
                "symbol": "FPT",
                "start_date": "2024-01-01",
                "end_date": "2024-01-31",
                "initial_cash": 100_000_000,
            },
        )) if hasattr(pytest, "run_async") else __import__("asyncio").run(svc.run_backtest(
            db=db,
            config={
                "strategy": strategy_config,
                "symbol": "FPT",
                "start_date": "2024-01-01",
                "end_date": "2024-01-31",
                "initial_cash": 100_000_000,
            },
        ))

        assert res["status"] == "failed"
        assert res["error_code"] == "INSUFFICIENT_WARMUP_HISTORY"
        assert "không đủ cho thời gian warmup" in res["message"]
        assert "yêu cầu tối thiểu 50 nến" in res["message"]
        assert "Unknown identifier" not in res["message"]
    finally:
        db.close()


def test_sqa_014_backtest_service_succeeds_with_adequate_warmup():
    """When candle history meets warmup requirements (>= 50 bars), backtest proceeds normally."""
    engine = create_engine("sqlite:///:memory:")
    Base.metadata.create_all(engine)
    Session = sessionmaker(bind=engine)
    db = Session()

    try:
        # 80 candles satisfies lookback of 50
        candles = _make_candles(80, symbol="FPT")
        for c in candles:
            db.add(c)
        db.commit()

        svc = BacktestService()
        strategy_config = {
            "name": "EMA Crossover",
            "version": "1.0",
            "indicators": [
                {"name": "ema_fast", "type": "ema", "length": 20},
                {"name": "ema_slow", "type": "ema", "length": 50},
            ],
            "entry_rules": [{"condition": "ema_fast > ema_slow and close > ema_fast"}],
            "exit_rules": [{"condition": "close < ema_slow"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        }

        res = __import__("asyncio").run(svc.run_backtest(
            db=db,
            config={
                "strategy": strategy_config,
                "symbol": "FPT",
                "start_date": "2024-01-01",
                "end_date": "2024-04-01",
                "initial_cash": 100_000_000,
            },
        ))

        assert res["status"] == "succeeded"
        assert "trades" in res
        assert isinstance(res["trades"], list)
    finally:
        db.close()
