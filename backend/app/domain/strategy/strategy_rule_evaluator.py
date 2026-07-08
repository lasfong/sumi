import numpy as np

from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    evaluate_condition,
    evaluate_rule_dsl,
    validate_condition,
    validate_rule_dsl,
)


class StrategyRuleEvaluator:
    """Shared strategy rule validation/evaluation for backtest and scanner paths."""

    @staticmethod
    def indicator_snapshot(indicator_values: dict, index: int) -> dict:
        snapshot = {}
        for key, values in indicator_values.items():
            value = values[index]
            snapshot[key] = float(value) if not np.isnan(value) else None
        return snapshot

    @staticmethod
    def validate_strategy_rules(strategy, indicator_names: set[str]) -> None:
        allowed_names = set(indicator_names)
        allowed_names.update(f"previous_{name}" for name in indicator_names)
        for rule in [*strategy.entry_rules, *strategy.exit_rules]:
            condition = rule.get("condition")
            if condition:
                validate_condition(condition, allowed_names)
            dsl_rule = rule.get("dsl") or StrategyRuleEvaluator.extract_inline_dsl(rule)
            if dsl_rule:
                validate_rule_dsl(dsl_rule, allowed_names)

    @staticmethod
    def evaluate_rules(rules, current, previous) -> bool:
        if not rules:
            return False

        for rule in rules:
            condition = rule.get("condition")
            dsl_rule = rule.get("dsl") or StrategyRuleEvaluator.extract_inline_dsl(rule)

            values = dict(current)
            values.update({f"previous_{key}": value for key, value in previous.items()})
            if condition and not evaluate_condition(condition, values):
                return False
            if dsl_rule and not evaluate_rule_dsl(dsl_rule, values):
                return False

        return True

    @staticmethod
    def extract_inline_dsl(rule: dict) -> dict | None:
        operators = {
            "all", "any", "not", "gt", "gte", "lt", "lte", "eq",
            "cross_up", "cross_down", "between", "rising", "falling",
        }
        inline_keys = [key for key in rule.keys() if key in operators]
        if not inline_keys:
            return None
        if len(inline_keys) > 1:
            raise RuleEvaluationError("Rule must contain only one inline DSL operator.")
        key = inline_keys[0]
        return {key: rule[key]}
