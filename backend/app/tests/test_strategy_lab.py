from datetime import date, timedelta

import pytest

from app.models.candle import Candle
from app.services.strategy_lab_service import StrategyLabService, SweepCancellationManager


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
    assert all("ranking_eligible" in row["metrics"] for row in result["variants"])
    assert all("metric_results" in row["metrics"] for row in result["variants"])
    assert all("equity_curve" not in row["response"].get("analytics", {}) for row in result["variants"])


@pytest.mark.asyncio
async def test_parameter_sweep_typed_parameters(db_session):
    base_date = date(2024, 1, 1)
    seed_candles(db_session, "SWEEP_TYPED", base_date)
    db_session.commit()

    config = {
        "symbol": "SWEEP_TYPED",
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=50)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Typed Parameter Strategy",
            "indicators": [
                {"name": "sma_fast", "type": "sma", "length": 5},
                {"name": "sma_slow", "type": "sma", "length": 20},
            ],
            "entry_rules": [{"dsl": {"cross_up": ["sma_fast", "sma_slow"]}}],
            "exit_rules": [{"dsl": {"cross_down": ["sma_fast", "sma_slow"]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [
            {
                "target_type": "indicator",
                "target_name": "sma_fast",
                "parameter": "length",
                "values": [3, 7],
            },
        ],
    }

    result = await StrategyLabService().run_parameter_sweep(db_session, config)

    assert result["status"] == "succeeded"
    assert result["total_variants"] == 2
    assert {row["parameters"]["indicators[0].length"] for row in result["variants"]} == {3, 7}
    assert result["variants"][0]["label"].startswith("sma_fast.length=")


def test_get_strategy_parameters():
    strategy = {
        "name": "Test Strategy",
        "indicators": [
            {"name": "ma_fast", "type": "ema", "length": 9},
            {"name": "bb", "type": "bbands", "length": 20, "std": 2.0},
            {"name": "macd_inst", "type": "macd", "fast": 12, "slow": 26, "signal": 9},
        ],
        "position_sizing": {"method": "fixed_quantity", "quantity": 200},
        "risk_management": {"stop_loss_pct": 5.0, "take_profit_pct": 10.0},
    }

    params = StrategyLabService.get_strategy_parameters(strategy)
    param_keys = [(p["target_name"], p["parameter"]) for p in params]

    assert ("ma_fast", "length") in param_keys
    assert ("bb", "length") in param_keys
    assert ("bb", "std") in param_keys
    assert ("macd_inst", "fast") in param_keys
    assert ("macd_inst", "slow") in param_keys
    assert ("macd_inst", "signal") in param_keys
    assert ("position_sizing", "quantity") in param_keys
    assert ("risk_management", "stop_loss_pct") in param_keys


def test_validate_strategy_definition():
    valid_strategy = {
        "name": "Valid Declarative Strategy",
        "version": "1.0",
        "indicators": [{"name": "rsi_main", "type": "rsi", "length": 14}],
        "entry_rules": [{"dsl": {"lt": ["rsi_main", 30]}}],
        "exit_rules": [{"dsl": {"gt": ["rsi_main", 70]}}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    validation = StrategyLabService.validate_strategy_definition(valid_strategy)
    assert validation["valid"] is True
    assert validation["errors"] == []

    invalid_strategy = {
        "name": "Invalid Strategy",
        "indicators": [{"name": "rsi_main", "type": "rsi", "length": 14}],
        "entry_rules": [{"condition": "__import__('os').system('ls')"}],  # Unsafe / illegal AST syntax
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    }
    invalid_validation = StrategyLabService.validate_strategy_definition(invalid_strategy)
    assert invalid_validation["valid"] is False
    assert len(invalid_validation["errors"]) > 0


def test_low_sample_variant_cannot_win_ranking():
    service = StrategyLabService()
    # Less than 5 trades should not be eligible for ranking (PRO-STRAT-05)
    low_sample = {
        "status": "succeeded",
        "analytics": {
            "total_trades": 3,
            "total_net_pnl": 50_000,
            "metrics": {"total_net_pnl": {"value": 50_000, "status": "valid", "sample_size": 3}},
        },
    }
    adequate_sample = {
        "status": "succeeded",
        "analytics": {
            "total_trades": 6,
            "total_net_pnl": 10_000,
            "metrics": {"total_net_pnl": {"value": 10_000, "status": "valid", "sample_size": 6}},
        },
    }

    low_metrics = service._extract_metrics(low_sample)
    adequate_metrics = service._extract_metrics(adequate_sample)

    assert low_metrics["ranking_eligible"] is False
    assert "Insufficient sample size (3 trades < 5 required)" in low_metrics["ranking_reason"]
    assert adequate_metrics["ranking_eligible"] is True
    assert adequate_metrics["ranking_reason"] is None


def test_robustness_scoring_classification():
    service = StrategyLabService()

    # In-Sample profitable, OOS profitable -> Robust
    is_resp = {"status": "succeeded", "analytics": {"total_trades": 10, "total_net_pnl": 1000, "profit_factor": 2.0, "metrics": {"total_net_pnl": {"status": "valid"}}}}
    oos_resp = {"status": "succeeded", "analytics": {"total_trades": 8, "total_net_pnl": 800, "profit_factor": 1.8, "metrics": {"total_net_pnl": {"status": "valid"}}}}
    robust_metrics = service._extract_metrics(is_resp, oos_resp)
    assert robust_metrics["robustness"]["badge"] == "Robust"
    assert robust_metrics["robustness"]["score"] > 50

    # In-Sample profitable, OOS loss -> Overfitted
    oos_loss = {"status": "succeeded", "analytics": {"total_trades": 8, "total_net_pnl": -300, "profit_factor": 0.6, "metrics": {"total_net_pnl": {"status": "valid"}}}}
    overfitted_metrics = service._extract_metrics(is_resp, oos_loss)
    assert overfitted_metrics["robustness"]["badge"] == "Overfitted"
    assert overfitted_metrics["robustness"]["score"] == 10.0


@pytest.mark.asyncio
async def test_parameter_sweep_with_oos_split(db_session):
    base_date = date(2023, 1, 1)
    seed_candles(db_session, "SWEEP_OOS", base_date, count=80)
    db_session.commit()

    config = {
        "symbol": "SWEEP_OOS",
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=40)),
        "oos_start_date": str(base_date + timedelta(days=40)),
        "oos_end_date": str(base_date + timedelta(days=80)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "OOS Split Strategy",
            "indicators": [{"name": "sma_test", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_test", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_test", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [{"path": "indicators[0].length", "values": [5, 10]}],
    }

    result = await StrategyLabService().run_parameter_sweep(db_session, config)

    assert result["status"] == "succeeded"
    assert result["in_sample_period"]["start_date"] == str(base_date)
    assert result["out_of_sample_period"]["start_date"] == str(base_date + timedelta(days=40))
    assert all("out_of_sample" in row["metrics"] for row in result["variants"])
    assert all("robustness" in row["metrics"] for row in result["variants"])


@pytest.mark.asyncio
async def test_parameter_sweep_rejects_overlapping_dates(db_session):
    config = {
        "symbol": "SWEEP_DATES",
        "start_date": "2024-01-01",
        "end_date": "2024-06-01",
        "oos_start_date": "2024-03-01",  # Overlaps with IS range (Jan to Jun)
        "oos_end_date": "2024-09-01",
        "strategy": {
            "name": "Overlap Strategy",
            "indicators": [{"name": "sma_test", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_test", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_test", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [{"path": "indicators[0].length", "values": [5]}],
    }

    with pytest.raises(ValueError, match="must not overlap"):
        await StrategyLabService().run_parameter_sweep(db_session, config)


@pytest.mark.asyncio
async def test_parameter_sweep_cancellation(db_session):
    base_date = date(2024, 1, 1)
    seed_candles(db_session, "SWEEP_CANCEL", base_date, count=50)
    db_session.commit()

    sweep_id = "test-cancellation-id-123"
    # Pre-cancel before run
    SweepCancellationManager.cancel_sweep(sweep_id)

    config = {
        "sweep_id": sweep_id,
        "symbol": "SWEEP_CANCEL",
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=50)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Cancel Strategy",
            "indicators": [{"name": "sma_test", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_test", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_test", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "sweep": [{"path": "indicators[0].length", "values": [5, 10, 15, 20]}],
    }

    result = await StrategyLabService().run_parameter_sweep(db_session, config)

    assert result["status"] == "cancelled"
    assert result["cancelled"] is True
    assert result["total_variants"] == 0  # Cancelled before first variant


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

    with pytest.raises(ValueError, match="values"):
        await StrategyLabService().run_parameter_sweep(db_session, config)
