import pytest
import math
import pandas as pd
from types import SimpleNamespace
from datetime import date, timedelta
from app.domain.engine.indicator_engine import IndicatorEngine
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.models.candle import Candle
from app.models.trade import Trade
from app.models.replay_session import ReplaySession
from app.domain.strategy.strategy_loader import list_available_strategies, load_strategy_from_dict
from app.services.backtest_service import BacktestService
from app.services.analytics_service import AnalyticsService


def test_backtest_supports_macd_rsi_strategy_indicators():
    strategies = list_available_strategies()
    sample = next(item for item in strategies if item["filename"] == "macd_rsi_momentum.yaml")
    strategy = load_strategy_from_dict(sample["config"])
    df = pd.DataFrame({
        "open": [100 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "high": [101 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "low": [99 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "close": [100 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "volume": [1_000_000 + i for i in range(80)],
    })

    values = StrategyIndicatorAdapter.compute(df, strategy.indicators)

    assert "macd_line" in values
    assert "macd_signal" in values
    assert "macd_hist" in values
    assert "rsi" in values
    StrategyRuleEvaluator.validate_strategy_rules(strategy, set(values.keys()))
    assert not hasattr(BacktestService(), "_compute_indicators")
    assert not hasattr(BacktestService(), "_validate_strategy_rules")
    assert not hasattr(BacktestService(), "_evaluate_rules")


def test_strategy_indicator_adapter_uses_shared_indicator_engine_outputs():
    strategies = list_available_strategies()
    sample = next(item for item in strategies if item["filename"] == "macd_rsi_momentum.yaml")
    strategy = load_strategy_from_dict(sample["config"])
    df = pd.DataFrame({
        "open": [100 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "high": [101 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "low": [99 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "close": [100 + math.sin(i / 3) * 5 + i * 0.2 for i in range(80)],
        "volume": [1_000_000 + i for i in range(80)],
    })

    values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
    macd_df = IndicatorEngine.compute(df, "macd", fast=12, slow=26, signal=9)
    rsi_df = IndicatorEngine.compute(df, "rsi", length=14)

    assert values["macd_line"][-1] == pytest.approx(macd_df["MACD_12_26_9"].iloc[-1])
    assert values["macd_signal"][-1] == pytest.approx(macd_df["MACDs_12_26_9"].iloc[-1])
    assert values["macd_hist"][-1] == pytest.approx(macd_df["MACDh_12_26_9"].iloc[-1])
    assert values["rsi"][-1] == pytest.approx(rsi_df["RSI_14"].iloc[-1])
    assert math.isnan(values["rsi"][0])
    candles = [SimpleNamespace(
        timestamp=date(2024, 1, 1) + timedelta(days=index),
        open=row.open, high=row.high, low=row.low, close=row.close, volume=row.volume,
    ) for index, row in df.iterrows()]
    assert BacktestService()._estimate_warmup(strategy, candles) == 33

@pytest.mark.asyncio
async def test_backtest_ma_crossover_e2e(db_session):
    symbol = "BACKTEST_TEST"
    base_date = date(2024, 1, 1)
    
    # Seed 100 candles with a sine wave pattern for clear MA crossovers
    candles_to_insert = []
    for i in range(100):
        price = 100 + 30 * math.sin(i * 0.06)
        c = Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=price - 0.5,
            high=price + 1,
            low=price - 1,
            close=price,
            volume=1000000
        )
        candles_to_insert.append(c)
        candles_to_insert.append(Candle(
            symbol="VNINDEX",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=1000 + i,
            high=1001 + i,
            low=999 + i,
            close=1000 + i,
            volume=1000000,
        ))
    db_session.add_all(candles_to_insert)
    db_session.commit()
    
    backtest_service = BacktestService()
    
    config = {
        "symbol": symbol,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=100)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "MA Crossover Test",
            "indicators": [
                {"name": "sma_short", "type": "sma", "length": 10},
                {"name": "sma_long", "type": "sma", "length": 30},
            ],
            "entry_rules": [
                {"condition": "sma_short > sma_long"},
            ],
            "exit_rules": [
                {"condition": "sma_short < sma_long"},
            ],
            "position_sizing": {
                "method": "fixed_quantity",
                "quantity": 1000,
            }
        }
    }
    
    result = await backtest_service.run_backtest(db_session, config)
    
    assert result["total_candles"] == 100
    assert result["analytics"] is not None
    assert result["data_coverage"]["requested_start"] == str(base_date)
    assert result["data_coverage"]["actual_start"].startswith(str(base_date))
    assert result["data_coverage"]["candle_count"] == 100
    assert result["data_coverage"]["warmup_candles"] == 29
    assert result["execution_assumptions"]["fees"] == {"buy_rate": 0.0015, "sell_rate": 0.0015}
    assert result["execution_assumptions"]["taxes"] == {"sell_tax_rate": 0.001}
    assert result["execution_assumptions"]["slippage"] == {"model": "none", "rate": 0.0}
    assert result["run_manifest"]["strategy_name"] == "MA Crossover Test"
    assert len(result["run_manifest"]["data_identity"]) == 64
    assert len(result["run_manifest"]["input_hash"]) == 64
    strategy = load_strategy_from_dict(config["strategy"])
    assumptions = backtest_service._build_assumptions(strategy)
    candles = db_session.query(Candle).filter(Candle.symbol == symbol).order_by(Candle.timestamp).all()
    same_inputs = backtest_service._build_manifest(strategy, symbol, candles, assumptions, initial_cash=100_000_000, benchmark_symbol="VNINDEX")
    different_cash = backtest_service._build_manifest(strategy, symbol, candles, assumptions, initial_cash=90_000_000, benchmark_symbol="VNINDEX")
    different_benchmark = backtest_service._build_manifest(strategy, symbol, candles, assumptions, initial_cash=100_000_000, benchmark_symbol="ALT_BENCH")
    assert same_inputs.input_hash != different_cash.input_hash
    assert same_inputs.input_hash != different_benchmark.input_hash
    persisted_session = db_session.query(ReplaySession).filter_by(id=result["session_id"]).one()
    assert persisted_session.current_index == result["total_candles"] - 1
    reopened_analytics = AnalyticsService.get_analytics(db_session, result["session_id"])
    assert reopened_analytics.metrics["sharpe_ratio"].sample_size == result["analytics"]["metrics"]["sharpe_ratio"]["sample_size"]
    assert any(row["group_type"] == "symbol" and row["key"] == symbol for row in result["slices"])
    assert any(row["group_type"] == "period" and row["key"] == "2024" for row in result["slices"])
    assert any(row["group_type"] == "regime" for row in result["slices"])
    
    # Check trades
    session_id = result["session_id"]
    trades = db_session.query(Trade).filter(Trade.session_id == session_id).all()
    
    # At least 1 trade should have occurred
    assert len(trades) >= 1
    
    closed_trades = [t for t in trades if t.status == "closed"]
    assert len(closed_trades) >= 1
    
    for trade in closed_trades:
        assert trade.result in ["win", "loss", "breakeven"]
        assert trade.net_pnl is not None
        assert trade.entry_price is not None
        assert trade.exit_price is not None


@pytest.mark.asyncio
async def test_backtest_declarative_rule_dsl_e2e(db_session):
    symbol = "BACKTEST_DSL"
    base_date = date(2024, 1, 1)

    candles_to_insert = []
    for i in range(100):
        price = 100 + 30 * math.sin(i * 0.06)
        candles_to_insert.append(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=price - 0.5,
            high=price + 1,
            low=price - 1,
            close=price,
            volume=1000000,
        ))
    db_session.add_all(candles_to_insert)
    db_session.commit()

    config = {
        "symbol": symbol,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=100)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Declarative DSL MA Crossover",
            "indicators": [
                {"name": "sma_short", "type": "sma", "length": 10},
                {"name": "sma_long", "type": "sma", "length": 30},
            ],
            "entry_rules": [
                {"dsl": {"gt": ["sma_short", "sma_long"]}},
            ],
            "exit_rules": [
                {"dsl": {"lt": ["sma_short", "sma_long"]}},
            ],
            "position_sizing": {
                "method": "fixed_quantity",
                "quantity": 1000,
            },
        },
    }

    result = await BacktestService().run_backtest(db_session, config)

    assert result["status"] == "succeeded"
    assert result["analytics"] is not None
    assert result["analytics"]["total_trades"] >= 1
    assert any(row["group_type"] == "symbol" and row["key"] == symbol for row in result["slices"])


@pytest.mark.asyncio
async def test_backtest_rejects_malicious_rule_expression(db_session):
    symbol = "BACKTEST_SECURITY"
    base_date = date(2024, 1, 1)

    for i in range(40):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=100 + i,
            high=101 + i,
            low=99 + i,
            close=100 + i,
            volume=1000000,
        ))
    db_session.commit()

    config = {
        "symbol": symbol,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=39)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Malicious Rule",
            "indicators": [
                {"name": "sma_short", "type": "sma", "length": 5},
                {"name": "sma_long", "type": "sma", "length": 10},
            ],
            "entry_rules": [
                {"condition": "__import__('os').system('echo unsafe')"},
            ],
            "exit_rules": [
                {"condition": "sma_short < sma_long"},
            ],
            "position_sizing": {
                "method": "fixed_quantity",
                "quantity": 100,
            },
        },
    }

    result = await BacktestService().run_backtest(db_session, config)

    assert result["status"] == "failed"
    assert result["error_code"] == "INVALID_RULE"
    assert result["analytics"] is None


@pytest.mark.asyncio
async def test_backtest_rejects_unknown_dsl_identifier(db_session):
    symbol = "BACKTEST_DSL_SECURITY"
    base_date = date(2024, 1, 1)

    for i in range(20):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=100 + i,
            high=101 + i,
            low=99 + i,
            close=100 + i,
            volume=1000000,
        ))
    db_session.commit()

    config = {
        "symbol": symbol,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=20)),
        "strategy": {
            "name": "Bad DSL Identifier",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["unknown_indicator", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_short", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
    }

    result = await BacktestService().run_backtest(db_session, config)

    assert result["status"] == "failed"
    assert result["error_code"] == "INVALID_RULE"
    assert "unknown_indicator" in result["message"]


@pytest.mark.asyncio
async def test_backtest_no_data_returns_failed_status(db_session):
    config = {
        "symbol": "MISSING_SYMBOL",
        "start_date": "2024-01-01",
        "end_date": "2024-02-01",
        "strategy": {
            "name": "No Data Strategy",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"condition": "sma_short > 0"}],
            "exit_rules": [{"condition": "sma_short < 0"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
    }

    result = await BacktestService().run_backtest(db_session, config)

    assert result["status"] == "failed"
    assert result["error_code"] == "NO_CANDLES"
    assert result["analytics"] is None


@pytest.mark.asyncio
async def test_backtest_runs_multiple_symbols_with_summary(db_session):
    base_date = date(2024, 1, 1)
    symbols = ["BT_AAA", "BT_BBB"]
    for symbol in symbols:
        for i in range(30):
            price = 100 + i
            db_session.add(Candle(
                symbol=symbol,
                timeframe="1D",
                timestamp=base_date + timedelta(days=i),
                open=price,
                high=price + 1,
                low=price - 1,
                close=price,
                volume=1000000,
            ))
    db_session.commit()

    config = {
        "symbols": symbols,
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=30)),
        "initial_cash": 100_000_000,
        "strategy": {
            "name": "Multi Symbol Smoke",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"condition": "sma_short > 0"}],
            "exit_rules": [{"condition": "sma_short < 0"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
    }

    result = await BacktestService().run_backtest(db_session, config)

    assert result["status"] == "succeeded"
    assert len(result["runs"]) == 2
    assert result["summary"]["total_symbols"] == 2
    assert result["summary"]["succeeded_symbols"] == 2
    assert result["summary"]["total_candles"] == 60
