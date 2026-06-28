from datetime import date, timedelta

import pytest

from app.models.candle import Candle
from app.services.strategy_lab_service import StrategyLabService


def seed_candles(db_session, symbol: str, base_date: date, count: int = 50):
    for index in range(count):
        price = 100 + index
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=index),
            open=price,
            high=price + 1,
            low=price - 1,
            close=price,
            volume=1000000,
        ))


@pytest.mark.asyncio
async def test_parameter_sweep_runs_variants(db_session):
    base_date = date(2024, 1, 1)
    seed_candles(db_session, "SWEEP_AAA", base_date)
    db_session.commit()

    config = {
        "symbol": "SWEEP_AAA",
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=50)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Sweep SMA Strategy",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_short", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_short", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [
            {"path": "indicators[0].length", "values": [5, 10]},
        ],
    }

    result = await StrategyLabService().run_parameter_sweep(db_session, config)

    assert result["status"] == "succeeded"
    assert result["total_variants"] == 2
    assert {row["parameters"]["indicators[0].length"] for row in result["variants"]} == {5, 10}
    assert all("net_pnl" in row["metrics"] for row in result["variants"])


@pytest.mark.asyncio
async def test_parameter_sweep_rejects_missing_values(db_session):
    config = {
        "symbol": "SWEEP_BAD",
        "start_date": "2024-01-01",
        "end_date": "2024-02-01",
        "strategy": {
            "name": "Bad Sweep",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_short", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_short", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [
            {"path": "indicators[0].length", "values": []},
        ],
    }

    with pytest.raises(ValueError, match="path and values"):
        await StrategyLabService().run_parameter_sweep(db_session, config)
