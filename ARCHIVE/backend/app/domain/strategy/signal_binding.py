"""Signal Binding Adapter for Strategy Rule Evaluator.

Connects precomputed signal snapshots to the safe AST RuleEvaluator.
Enforces fail-closed availability and quality semantics: if any referenced signal
dependency is unavailable or invalid, evaluation halts immediately and returns a
nullable result with explanation, preventing 'not', 'any', or 'eq False' from
evaluating missing data to True.

Does NOT modify or widen the AST whitelist in rule_evaluator.py.
"""

import ast
from dataclasses import dataclass
import math
from typing import Any, Dict, List, Optional, Set, Tuple, Union


from app.domain.signals.models import (
    SignalOutputPoint,
    SignalOutputType,
    SignalQuality,
    to_finite_float,
)
from app.domain.signals.registry import SignalRegistry
from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    evaluate_condition,
    evaluate_rule_dsl,
    get_rule_dsl_identifiers,
    get_rule_identifiers,
    validate_condition,
    validate_rule_dsl,
)



def _extract_cross_operands(rule: Any) -> List[Tuple[str, Any, Any]]:
    """Extract all (operator, left, right) from cross_up and cross_down DSL nodes."""
    if not isinstance(rule, dict) or len(rule) != 1:
        return []
    op, args = next(iter(rule.items()))
    if op in {"cross_up", "cross_down"}:
        if isinstance(args, list) and len(args) == 2:
            return [(op, args[0], args[1])]
        return []
    if op in {"all", "any"}:
        if isinstance(args, list):
            res = []
            for item in args:
                res.extend(_extract_cross_operands(item))
            return res
    if op == "not":
        return _extract_cross_operands(args)
    return []


def _validate_snapshot_integrity(
    point: SignalOutputPoint,
    alias: str,
    prefix: str = "",
) -> Optional["SignalBindingResult"]:
    """Validate that a VALID signal snapshot conforms to availability metadata and registry type contracts.

    Returns None if valid; returns fail-closed SignalBindingResult with INVALID_VOLUME if malformed.
    """
    if point.available_at_index != point.bar_index or point.availability_event != "BAR_CLOSE":
        return SignalBindingResult(
            value=None,
            is_valid=False,
            quality=SignalQuality.INVALID_VOLUME,
            reason=f"INVALID_AVAILABILITY_METADATA_{prefix}{alias}",
        )

    sig_name = SignalRegistry.resolve_name_from_alias(alias)
    defn = SignalRegistry.get_definition(sig_name) if sig_name else None
    if defn is None:
        return SignalBindingResult(
            value=None,
            is_valid=False,
            quality=SignalQuality.INVALID_VOLUME,
            reason=f"UNKNOWN_REGISTRY_DEFINITION_{prefix}{alias}",
        )

    expected_output_type = defn.output_type.value if hasattr(defn.output_type, "value") else str(defn.output_type)
    actual_output_type = point.output_type.value if hasattr(point.output_type, "value") else str(point.output_type)

    if actual_output_type != expected_output_type:
        return SignalBindingResult(
            value=None,
            is_valid=False,
            quality=SignalQuality.INVALID_VOLUME,
            reason=f"MISMATCHED_OUTPUT_TYPE_{prefix}{alias}",
        )

    if expected_output_type == "bool":
        if not isinstance(point.value, bool):
            return SignalBindingResult(
                value=None,
                is_valid=False,
                quality=SignalQuality.INVALID_VOLUME,
                reason=f"MALFORMED_SIGNAL_VALUE_{prefix}{alias}_EXPECTED_BOOL",
            )
    elif expected_output_type == "float":
        if to_finite_float(point.value) is None:
            return SignalBindingResult(
                value=None,
                is_valid=False,
                quality=SignalQuality.INVALID_VOLUME,
                reason=f"MALFORMED_SIGNAL_VALUE_{prefix}{alias}_EXPECTED_FLOAT",
            )
    elif expected_output_type == "enum":
        if isinstance(point.value, bool) or not isinstance(point.value, str):
            return SignalBindingResult(
                value=None,
                is_valid=False,
                quality=SignalQuality.INVALID_VOLUME,
                reason=f"MALFORMED_SIGNAL_VALUE_{prefix}{alias}_EXPECTED_ENUM",
            )
    else:
        if point.value is None:
            return SignalBindingResult(
                value=None,
                is_valid=False,
                quality=SignalQuality.INVALID_VOLUME,
                reason=f"MALFORMED_SIGNAL_VALUE_{prefix}{alias}",
            )

    return None


@dataclass(frozen=True)
class SignalBindingResult:
    """Outcome of evaluating a strategy rule over precomputed signal outputs."""
    value: Optional[bool]
    is_valid: bool
    quality: SignalQuality
    reason: str


class SignalBindingAdapter:
    """Adapter evaluating rules over precomputed signal snapshots with quality guards."""

    @classmethod
    def get_supported_signal_aliases(cls) -> Set[str]:
        """Return all supported AST alias identifiers from registered signals."""
        aliases = set()
        for defn in SignalRegistry.list_definitions():
            if defn.ast_alias:
                aliases.add(defn.ast_alias)
        return aliases

    @classmethod
    def validate_rule(
        cls,
        rule: Union[str, Dict[str, Any]],
        allowed_extra_names: Optional[Set[str]] = None,
        flow_constraints: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Validate that a rule (AST string or DSL dict) only references recognized identifiers and adheres to method constraints."""
        allowed = cls.get_supported_signal_aliases()
        if allowed_extra_names:
            allowed = allowed | allowed_extra_names

        identifiers: Set[str] = set()

        if isinstance(rule, str):
            # Check for illegal dotted access attempt (e.g. volume.spike instead of volume__spike)
            try:
                tree = ast.parse(rule, mode="eval")
            except SyntaxError as e:
                raise RuleEvaluationError(f"Invalid rule syntax: {e}")

            for node in ast.walk(tree):
                if isinstance(node, ast.Attribute):
                    raise RuleEvaluationError(
                        "Dotted signal names like 'volume.spike' are not allowed in rule expressions. "
                        "Use explicit AST alias 'volume__spike'."
                    )
            validate_condition(rule, allowed)
            identifiers = get_rule_identifiers(rule)
        elif isinstance(rule, dict):
            for ident in get_rule_dsl_identifiers(rule):
                if "." in ident:
                    raise RuleEvaluationError(
                        f"Dotted signal names like '{ident}' are not allowed in rule expressions. "
                        f"Use explicit AST alias '{ident.replace('.', '__')}'."
                    )
            validate_rule_dsl(rule, allowed)
            identifiers = get_rule_dsl_identifiers(rule)
        else:
            raise RuleEvaluationError(f"Unsupported rule type: {type(rule).__name__}")

        # Method and Quality constraints guardrail (BBI-SIG-003, FR-CORE-014)
        has_bb_signal = any(ident.startswith("bb__") for ident in identifiers)
        if flow_constraints or has_bb_signal:
            constraints = flow_constraints or {}
            accepted_methods = constraints.get("accepted_methods", ["OHLCV_PROXY"])
            for m in accepted_methods:
                m_str = m.upper() if isinstance(m, str) else str(m)
                if m_str != "OHLCV_PROXY":
                    raise RuleEvaluationError(
                        f"Flow method '{m}' is not authorized for daily Symbol BB. "
                        f"Only OHLCV_PROXY is validated. True Flow and Tick Test remain R&D capabilities."
                    )
            min_quality = constraints.get("min_quality", "HIGH")
            valid_qualities = {"HIGH", "MEDIUM", "DEGRADED", "SUSPECT", "INVALID"}
            if min_quality.upper() not in valid_qualities:
                raise RuleEvaluationError(f"Invalid min_quality constraint '{min_quality}'")

    @classmethod
    def evaluate(
        cls,
        rule: Union[str, Dict[str, Any]],
        signal_points: Dict[str, SignalOutputPoint],
        extra_values: Optional[Dict[str, Any]] = None,
        previous_signal_points: Optional[Dict[str, SignalOutputPoint]] = None,
    ) -> SignalBindingResult:
        """Safely evaluate a rule over precomputed signal points for a specific bar.
        
        Args:
            rule: AST string (e.g. 'volume__spike == True') or DSL dict (e.g. {'eq': ['volume__spike', True]}).
            signal_points: Map of {ast_alias: SignalOutputPoint} for the bar being evaluated.
            extra_values: Optional additional numeric/boolean values (e.g. indicators/candles).
            previous_signal_points: Optional map of {ast_alias: SignalOutputPoint} for the previous bar (bar t-1),
                required for cross_up / cross_down operators referencing registered signal aliases.
            
        Returns:
            SignalBindingResult indicating whether evaluation was valid and the resulting boolean value.
        """
        # 1. Parse identifiers and cross operands
        cross_signal_aliases: Set[str] = set()
        if isinstance(rule, str):
            try:
                tree = ast.parse(rule, mode="eval")
            except SyntaxError as e:
                raise RuleEvaluationError(f"Invalid rule syntax: {e}")

            for node in ast.walk(tree):
                if isinstance(node, ast.Attribute):
                    raise RuleEvaluationError(
                        "Dotted signal names like 'volume.spike' are not allowed in rule expressions. "
                        "Use explicit AST alias 'volume__spike'."
                    )
            try:
                identifiers = get_rule_identifiers(rule)
            except Exception as e:
                raise RuleEvaluationError(f"Invalid rule syntax: {e}")

        elif isinstance(rule, dict):
            try:
                identifiers = get_rule_dsl_identifiers(rule)
            except Exception as e:
                raise RuleEvaluationError(f"Invalid rule DSL: {e}")

            for ident in identifiers:
                if "." in ident:
                    raise RuleEvaluationError(
                        f"Dotted signal names like '{ident}' are not allowed in rule expressions. "
                        f"Use explicit AST alias '{ident.replace('.', '__')}'."
                    )

            supported_aliases = cls.get_supported_signal_aliases()
            for _op, left, right in _extract_cross_operands(rule):
                if isinstance(left, str) and left in supported_aliases:
                    cross_signal_aliases.add(left)
                if isinstance(right, str) and right in supported_aliases:
                    cross_signal_aliases.add(right)
        else:
            raise RuleEvaluationError(f"Unsupported rule type: {type(rule).__name__}")

        # 2. Check for unknown identifiers
        supported_signal_aliases = cls.get_supported_signal_aliases()
        extra_names = set(extra_values.keys()) if extra_values else set()
        allowed = supported_signal_aliases | extra_names
        unknown = identifiers - allowed
        if unknown:
            raise RuleEvaluationError(f"Unknown identifier(s) in rule: {', '.join(sorted(unknown))}")

        # 3. Guard: Check quality & availability for ALL referenced signal dependencies (current bar)
        values: Dict[str, Any] = {}

        for ident in identifiers:
            if ident in supported_signal_aliases:
                if ident not in signal_points:
                    return SignalBindingResult(
                        value=None,
                        is_valid=False,
                        quality=SignalQuality.INSUFFICIENT_HISTORY,
                        reason=f"MISSING_SIGNAL_DEPENDENCY_{ident}",
                    )

                point = signal_points[ident]
                if point.quality != SignalQuality.VALID:
                    # Signal is unavailable/invalid (e.g. INSUFFICIENT_HISTORY, ZERO_BASELINE, INVALID_VOLUME)
                    # Abort evaluation immediately - do NOT call evaluator!
                    return SignalBindingResult(
                        value=None,
                        is_valid=False,
                        quality=point.quality,
                        reason=f"DEPENDENCY_UNAVAILABLE_{ident}_{point.quality.value}",
                    )

                # Snapshot integrity validation for VALID current point
                integrity_error = _validate_snapshot_integrity(point, ident)
                if integrity_error is not None:
                    return integrity_error

                values[ident] = point.value
            elif extra_values and ident in extra_values:
                extra_val = extra_values[ident]
                if extra_val is None:
                    return SignalBindingResult(
                        value=None,
                        is_valid=False,
                        quality=SignalQuality.INSUFFICIENT_HISTORY,
                        reason=f"MISSING_EXTRA_VALUE_{ident}",
                    )
                values[ident] = extra_val

        # 4. Guard: Check quality & availability for previous signal points of cross operands
        for alias in sorted(cross_signal_aliases):
            if previous_signal_points is None or alias not in previous_signal_points:
                return SignalBindingResult(
                    value=None,
                    is_valid=False,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reason=f"MISSING_PREVIOUS_SIGNAL_DEPENDENCY_{alias}",
                )

            curr_point = signal_points[alias]
            prev_point = previous_signal_points[alias]

            # Adjacency check: previous point must be exactly current.bar_index - 1
            # Missing, future, same-bar, or non-adjacent fail closed with INSUFFICIENT_HISTORY
            if prev_point.bar_index != curr_point.bar_index - 1:
                return SignalBindingResult(
                    value=None,
                    is_valid=False,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reason=f"NON_ADJACENT_PREVIOUS_SIGNAL_{alias}",
                )

            if prev_point.quality != SignalQuality.VALID:
                return SignalBindingResult(
                    value=None,
                    is_valid=False,
                    quality=prev_point.quality,
                    reason=f"DEPENDENCY_UNAVAILABLE_previous_{alias}_{prev_point.quality.value}",
                )

            # Snapshot integrity validation for VALID previous point
            prev_integrity_error = _validate_snapshot_integrity(prev_point, alias, prefix="previous_")
            if prev_integrity_error is not None:
                return prev_integrity_error

            values[f"previous_{alias}"] = prev_point.value


        # 5. All dependencies are available and valid: execute safe evaluator
        if isinstance(rule, str):
            result = evaluate_condition(rule, values)
        else:
            result = evaluate_rule_dsl(rule, values)

        return SignalBindingResult(
            value=bool(result),
            is_valid=True,
            quality=SignalQuality.VALID,
            reason="EVALUATED",
        )
