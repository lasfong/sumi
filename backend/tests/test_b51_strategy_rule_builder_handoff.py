"""B51 tests for StrategyRuleBuilder AST alias resolution and BacktestService execution (GAP-U-03)."""

from datetime import datetime, timedelta
import pandas as pd
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator


def _make_sample_df(rows: int = 80) -> pd.DataFrame:
    base_date = datetime(2024, 1, 2)
    records = []
    price = 25000.0
    for idx in range(rows):
        delta = -250.0 if (idx % 14) < 6 else 320.0
        price = max(10000.0, price + delta)
        records.append(
            {
                "timestamp": (base_date + timedelta(days=idx)).strftime("%Y-%m-%d"),
                "open": price - 100.0,
                "high": price + 250.0,
                "low": price - 250.0,
                "close": price,
                "volume": 1_500_000 + (idx % 5) * 200_000,
            }
        )
    return pd.DataFrame(records)


def test_b51_custom_ast_rule_resolves_signal_registry_aliases_in_adapter_and_evaluator():
    """Custom AST rules from StrategyRuleBuilder with type='signal' resolve SignalRegistry aliases."""
    payload = {
        "name": "Custom RSI + Price Action Rule",
        "version": "1.0",
        "description": "Created from StrategyRuleBuilder UI",
        "indicators": [
            {"name": "rsi_14", "type": "signal"},
            {"name": "ema_20", "type": "signal"},
        ],
        "entry_rules": [{"condition": "rsi_14 < 85 and ema_20 > 1000"}],
        "exit_rules": [{"condition": "rsi_14 > 90"}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
        "risk_management": {"stop_loss_pct": 7.0, "take_profit_pct": 15.0},
    }

    strategy = load_strategy_from_dict(payload)
    df = _make_sample_df(80)
    indicator_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)

    assert "rsi_14" in indicator_values
    assert "ema_20" in indicator_values
    StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))

    current = StrategyRuleEvaluator.indicator_snapshot(indicator_values, len(df) - 1)
    previous = StrategyRuleEvaluator.indicator_snapshot(indicator_values, len(df) - 2)
    assert current["rsi_14"] is not None
    assert current["ema_20"] is not None
    assert StrategyRuleEvaluator.evaluate_rules(strategy.entry_rules, current, previous) is True


def test_b51_boolean_signal_registry_alias_evaluates_in_ast_rule():
    """Boolean SignalRegistry aliases (e.g., volume_spike / bull_engulfing) preserve bool in snapshot."""
    payload = {
        "name": "Boolean Signal Rule",
        "version": "1.0",
        "description": "Boolean alias test",
        "indicators": [
            {"name": "volume_spike", "type": "signal"},
            {"name": "rsi_14", "type": "signal"},
        ],
        "entry_rules": [{"condition": "volume_spike == False and rsi_14 > 10"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
        "risk_management": {"stop_loss_pct": 7.0, "take_profit_pct": 15.0},
    }

    strategy = load_strategy_from_dict(payload)
    df = _make_sample_df(60)
    indicator_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
    assert "volume_spike" in indicator_values
    assert "rsi_14" in indicator_values
    StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))

    current = StrategyRuleEvaluator.indicator_snapshot(indicator_values, len(df) - 1)
    previous = StrategyRuleEvaluator.indicator_snapshot(indicator_values, len(df) - 2)
    assert isinstance(current["volume_spike"], bool)
    assert StrategyRuleEvaluator.evaluate_rules(strategy.entry_rules, current, previous) is True
