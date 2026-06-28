import ast
import math
from typing import Any


class RuleEvaluationError(ValueError):
    """Raised when a strategy rule uses unsupported or unsafe syntax."""


_COMPARE_OPERATORS = (
    ast.Gt,
    ast.GtE,
    ast.Lt,
    ast.LtE,
    ast.Eq,
    ast.NotEq,
)


def get_rule_identifiers(condition: str) -> set[str]:
    tree = _parse_condition(condition)
    return {node.id for node in ast.walk(tree) if isinstance(node, ast.Name)}


def validate_condition(condition: str, allowed_names: set[str]) -> None:
    identifiers = get_rule_identifiers(condition)
    unknown = identifiers - allowed_names
    if unknown:
        raise RuleEvaluationError(f"Unknown identifier(s): {', '.join(sorted(unknown))}")


def evaluate_condition(condition: str, values: dict[str, Any]) -> bool:
    tree = _parse_condition(condition)
    return bool(_evaluate(tree.body, values))


def validate_rule_dsl(rule: Any, allowed_names: set[str]) -> None:
    identifiers = get_rule_dsl_identifiers(rule)
    unknown = identifiers - allowed_names
    if unknown:
        raise RuleEvaluationError(f"Unknown identifier(s): {', '.join(sorted(unknown))}")


def get_rule_dsl_identifiers(rule: Any) -> set[str]:
    if isinstance(rule, str):
        return {rule}
    if isinstance(rule, (int, float, bool)) or rule is None:
        return set()
    if isinstance(rule, list):
        identifiers = set()
        for item in rule:
            identifiers.update(get_rule_dsl_identifiers(item))
        return identifiers
    if not isinstance(rule, dict):
        raise RuleEvaluationError(f"Unsupported rule DSL node: {type(rule).__name__}")

    if len(rule) != 1:
        raise RuleEvaluationError("Rule DSL node must contain exactly one operator.")

    operator, args = next(iter(rule.items()))
    if operator in {"all", "any"}:
        if not isinstance(args, list):
            raise RuleEvaluationError(f"Operator '{operator}' expects a list.")
        identifiers = set()
        for item in args:
            identifiers.update(get_rule_dsl_identifiers(item))
        return identifiers

    if operator == "not":
        return get_rule_dsl_identifiers(args)

    if operator in {"gt", "gte", "lt", "lte", "eq", "cross_up", "cross_down", "between", "rising", "falling"}:
        return get_rule_dsl_identifiers(args)

    raise RuleEvaluationError(f"Unsupported rule DSL operator: {operator}")


def evaluate_rule_dsl(rule: Any, values: dict[str, Any]) -> bool:
    if isinstance(rule, bool):
        return rule
    if not isinstance(rule, dict) or len(rule) != 1:
        raise RuleEvaluationError("Rule DSL node must be an object with one operator.")

    operator, args = next(iter(rule.items()))

    if operator == "all":
        _require_list(operator, args)
        return all(evaluate_rule_dsl(item, values) for item in args)
    if operator == "any":
        _require_list(operator, args)
        return any(evaluate_rule_dsl(item, values) for item in args)
    if operator == "not":
        return not evaluate_rule_dsl(args, values)

    if operator in {"gt", "gte", "lt", "lte", "eq"}:
        left, right = _binary_args(operator, args)
        left_value = _resolve_value(left, values)
        right_value = _resolve_value(right, values)
        if _is_missing_number(left_value) or _is_missing_number(right_value):
            return False
        if operator == "gt":
            return left_value > right_value
        if operator == "gte":
            return left_value >= right_value
        if operator == "lt":
            return left_value < right_value
        if operator == "lte":
            return left_value <= right_value
        return left_value == right_value

    if operator in {"cross_up", "cross_down"}:
        left, right = _binary_args(operator, args)
        current_left = _resolve_value(left, values)
        current_right = _resolve_value(right, values)
        previous_left = _resolve_value(f"previous_{left}" if isinstance(left, str) else left, values)
        previous_right = _resolve_value(f"previous_{right}" if isinstance(right, str) else right, values)
        if any(_is_missing_number(value) for value in [current_left, current_right, previous_left, previous_right]):
            return False
        if operator == "cross_up":
            return previous_left <= previous_right and current_left > current_right
        return previous_left >= previous_right and current_left < current_right

    if operator == "between":
        if not isinstance(args, list) or len(args) != 3:
            raise RuleEvaluationError("Operator 'between' expects three arguments.")
        value = _resolve_value(args[0], values)
        lower = _resolve_value(args[1], values)
        upper = _resolve_value(args[2], values)
        if any(_is_missing_number(item) for item in [value, lower, upper]):
            return False
        return lower <= value <= upper

    if operator in {"rising", "falling"}:
        value_ref = _single_arg(operator, args)
        if not isinstance(value_ref, str):
            raise RuleEvaluationError(f"Operator '{operator}' expects an indicator name.")
        current = _resolve_value(value_ref, values)
        previous = _resolve_value(f"previous_{value_ref}", values)
        if _is_missing_number(current) or _is_missing_number(previous):
            return False
        return current > previous if operator == "rising" else current < previous

    raise RuleEvaluationError(f"Unsupported rule DSL operator: {operator}")


def _parse_condition(condition: str) -> ast.Expression:
    try:
        tree = ast.parse(condition, mode="eval")
    except SyntaxError as exc:
        raise RuleEvaluationError(f"Invalid rule syntax: {condition}") from exc

    for node in ast.walk(tree):
        if isinstance(node, (ast.Expression, ast.Load, ast.Name, ast.Constant)):
            continue
        if isinstance(node, (ast.BoolOp, ast.And, ast.Or, ast.UnaryOp, ast.Not, ast.USub)):
            continue
        if isinstance(node, (ast.Compare, *_COMPARE_OPERATORS)):
            continue
        raise RuleEvaluationError(f"Unsupported rule syntax: {type(node).__name__}")

    return tree


def _evaluate(node: ast.AST, values: dict[str, Any]) -> Any:
    if isinstance(node, ast.Constant):
        return node.value

    if isinstance(node, ast.Name):
        if node.id not in values:
            raise RuleEvaluationError(f"Unknown identifier: {node.id}")
        return values[node.id]

    if isinstance(node, ast.UnaryOp):
        operand = _evaluate(node.operand, values)
        if isinstance(node.op, ast.Not):
            return not bool(operand)
        if isinstance(node.op, ast.USub):
            return -float(operand)
        raise RuleEvaluationError(f"Unsupported unary operator: {type(node.op).__name__}")

    if isinstance(node, ast.BoolOp):
        if isinstance(node.op, ast.And):
            return all(bool(_evaluate(value, values)) for value in node.values)
        if isinstance(node.op, ast.Or):
            return any(bool(_evaluate(value, values)) for value in node.values)
        raise RuleEvaluationError(f"Unsupported boolean operator: {type(node.op).__name__}")

    if isinstance(node, ast.Compare):
        left = _evaluate(node.left, values)
        for op, comparator in zip(node.ops, node.comparators):
            right = _evaluate(comparator, values)
            if not _compare(left, right, op):
                return False
            left = right
        return True

    raise RuleEvaluationError(f"Unsupported rule node: {type(node).__name__}")


def _compare(left: Any, right: Any, op: ast.cmpop) -> bool:
    if _is_missing_number(left) or _is_missing_number(right):
        return False

    if isinstance(op, ast.Gt):
        return left > right
    if isinstance(op, ast.GtE):
        return left >= right
    if isinstance(op, ast.Lt):
        return left < right
    if isinstance(op, ast.LtE):
        return left <= right
    if isinstance(op, ast.Eq):
        return left == right
    if isinstance(op, ast.NotEq):
        return left != right

    raise RuleEvaluationError(f"Unsupported comparison operator: {type(op).__name__}")


def _is_missing_number(value: Any) -> bool:
    return value is None or (isinstance(value, float) and math.isnan(value))


def _resolve_value(value: Any, values: dict[str, Any]) -> Any:
    if isinstance(value, str):
        if value not in values:
            raise RuleEvaluationError(f"Unknown identifier: {value}")
        return values[value]
    return value


def _require_list(operator: str, args: Any) -> None:
    if not isinstance(args, list):
        raise RuleEvaluationError(f"Operator '{operator}' expects a list.")


def _binary_args(operator: str, args: Any) -> tuple[Any, Any]:
    if not isinstance(args, list) or len(args) != 2:
        raise RuleEvaluationError(f"Operator '{operator}' expects two arguments.")
    return args[0], args[1]


def _single_arg(operator: str, args: Any) -> Any:
    if isinstance(args, list):
        if len(args) != 1:
            raise RuleEvaluationError(f"Operator '{operator}' expects one argument.")
        return args[0]
    return args
