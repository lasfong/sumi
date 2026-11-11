import sys
from pathlib import Path
import pytest

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    validate_condition,
    evaluate_condition,
    _compare,
)
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
import ast


def test_sqa_010_valid_numeric_comparison_passes():
    """Valid numeric comparison conditions pass AST validation."""
    allowed = {"volume", "rsi_14", "close", "macd_line"}
    validate_condition("volume > 1000", allowed)
    validate_condition("rsi_14 <= 30.5", allowed)
    validate_condition("macd_line >= -2.5", allowed)
    validate_condition("close == 25000", allowed)
    validate_condition("volume != 0", allowed)


def test_sqa_010_valid_boolean_comparison_passes():
    """Valid boolean comparison conditions pass AST validation."""
    allowed = {"volume_spike", "is_bullish"}
    validate_condition("volume_spike == True", allowed)
    validate_condition("is_bullish == False", allowed)


def test_sqa_010_unknown_identifier_in_comparison_fails():
    """Non-numeric unquoted strings in comparison are parsed as unknown identifiers and rejected."""
    allowed = {"volume", "close"}
    with pytest.raises(RuleEvaluationError) as exc_info:
        validate_condition("volume >= abc_invalid", allowed)
    assert "Unknown identifier(s): abc_invalid" in str(exc_info.value)


def test_sqa_010_string_literal_in_comparison_fails():
    """String literals in comparison operands are rejected with clear error."""
    allowed = {"close"}
    with pytest.raises(RuleEvaluationError) as exc_info:
        validate_condition("close > 'invalid_string'", allowed)
    assert "Comparison operand must be numeric or boolean" in str(exc_info.value)


def test_sqa_010_compare_handles_type_error_gracefully():
    """_compare returns False rather than crashing when comparing incompatible types."""
    # Comparing string and float should return False, not raise TypeError
    assert _compare("text", 123.4, ast.Gt()) is False
    assert _compare(123.4, "text", ast.Lt()) is False
    assert _compare(None, 100, ast.Gt()) is False
    assert _compare(float("nan"), 100, ast.Gt()) is False


def test_sqa_010_strategy_rule_evaluator_integration():
    """StrategyRuleEvaluator.validate_strategy_rules rejects invalid comparison values."""
    allowed_indicators = {"volume", "rsi_14"}

    valid_strategy = load_strategy_from_dict({
        "name": "Valid Strategy",
        "indicators": [{"name": "volume", "type": "signal"}, {"name": "rsi_14", "type": "signal"}],
        "entry_rules": [{"condition": "volume > 500000 and rsi_14 < 30"}],
        "exit_rules": [{"condition": "rsi_14 > 70"}],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    StrategyRuleEvaluator.validate_strategy_rules(valid_strategy, allowed_indicators)

    invalid_strategy = load_strategy_from_dict({
        "name": "Invalid Strategy",
        "indicators": [{"name": "volume", "type": "signal"}],
        "entry_rules": [{"condition": "volume > abc_invalid"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    with pytest.raises(RuleEvaluationError) as exc_info:
        StrategyRuleEvaluator.validate_strategy_rules(invalid_strategy, allowed_indicators)
    assert "Unknown identifier(s): abc_invalid" in str(exc_info.value)


def test_sqa_010_evaluate_condition_numeric():
    """evaluate_condition correctly evaluates numeric AST conditions with real values."""
    values = {"volume": 600000, "close": 25000, "rsi": 28.5}
    assert evaluate_condition("volume > 500000 and rsi < 30", values) is True
    assert evaluate_condition("volume > 1000000 or close < 20000", values) is False
