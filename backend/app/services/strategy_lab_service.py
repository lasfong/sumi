from copy import deepcopy
from datetime import datetime
from itertools import product
import math
import re
from typing import Any, Dict, List, Optional
import uuid
from sqlalchemy.orm import Session

from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.services.backtest_service import BacktestService


class SweepCancellationManager:
    _cancelled_sweeps: set[str] = set()
    _active_sweeps: set[str] = set()

    @classmethod
    def register_sweep(cls, sweep_id: str):
        cls._active_sweeps.add(sweep_id)

    @classmethod
    def cancel_sweep(cls, sweep_id: str):
        cls._cancelled_sweeps.add(sweep_id)

    @classmethod
    def is_cancelled(cls, sweep_id: str) -> bool:
        return sweep_id in cls._cancelled_sweeps

    @classmethod
    def cleanup_sweep(cls, sweep_id: str):
        cls._active_sweeps.discard(sweep_id)
        cls._cancelled_sweeps.discard(sweep_id)


class StrategyLabService:
    def __init__(self):
        self.backtest_service = BacktestService()

    @staticmethod
    def get_strategy_parameters(strategy: dict) -> list[dict]:
        """Extract inspectable/typed parameter definitions from a strategy dict."""
        params = []
        # Indicators
        for idx, ind in enumerate(strategy.get("indicators", [])):
            ind_name = ind.get("name", f"indicator_{idx}")
            ind_type = (ind.get("type") or "").lower()

            if ind_type in {"sma", "ema", "rsi", "atr", "cci", "mfi", "adx"}:
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "length",
                    "label": f"{ind_name} ({ind_type.upper()}) Length",
                    "type": "int",
                    "current_value": ind.get("length", 14),
                    "path": f"indicators[{idx}].length",
                })
            elif ind_type == "macd":
                for p in ["fast", "slow", "signal"]:
                    params.append({
                        "target_type": "indicator",
                        "target_name": ind_name,
                        "parameter": p,
                        "label": f"{ind_name} MACD {p.capitalize()}",
                        "type": "int",
                        "current_value": ind.get(p, 12 if p == "fast" else (26 if p == "slow" else 9)),
                        "path": f"indicators[{idx}].{p}",
                    })
            elif ind_type == "bbands":
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "length",
                    "label": f"{ind_name} BBands Length",
                    "type": "int",
                    "current_value": ind.get("length", 20),
                    "path": f"indicators[{idx}].length",
                })
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "std",
                    "label": f"{ind_name} BBands Std Dev",
                    "type": "float",
                    "current_value": ind.get("std", 2.0),
                    "path": f"indicators[{idx}].std",
                })
            elif ind_type == "stoch":
                for p in ["k", "d", "smooth_k"]:
                    params.append({
                        "target_type": "indicator",
                        "target_name": ind_name,
                        "parameter": p,
                        "label": f"{ind_name} Stoch {p.upper()}",
                        "type": "int",
                        "current_value": ind.get(p, 14 if p == "k" else 3),
                        "path": f"indicators[{idx}].{p}",
                    })
            elif ind_type == "kc":
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "length",
                    "label": f"{ind_name} KC Length",
                    "type": "int",
                    "current_value": ind.get("length", 20),
                    "path": f"indicators[{idx}].length",
                })
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "scalar",
                    "label": f"{ind_name} KC Multiplier",
                    "type": "float",
                    "current_value": ind.get("scalar", 2.0),
                    "path": f"indicators[{idx}].scalar",
                })
            elif ind_type == "psar":
                for p in ["af0", "af", "max_af"]:
                    params.append({
                        "target_type": "indicator",
                        "target_name": ind_name,
                        "parameter": p,
                        "label": f"{ind_name} PSAR {p}",
                        "type": "float",
                        "current_value": ind.get(p, 0.02 if p in {"af0", "af"} else 0.2),
                        "path": f"indicators[{idx}].{p}",
                    })
            elif ind_type == "supertrend":
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "length",
                    "label": f"{ind_name} SuperTrend Length",
                    "type": "int",
                    "current_value": ind.get("length", 10),
                    "path": f"indicators[{idx}].length",
                })
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "multiplier",
                    "label": f"{ind_name} SuperTrend Multiplier",
                    "type": "float",
                    "current_value": ind.get("multiplier", 3.0),
                    "path": f"indicators[{idx}].multiplier",
                })
            elif ind_type == "ichimoku":
                for p in ["tenkan", "kijun", "senkou"]:
                    params.append({
                        "target_type": "indicator",
                        "target_name": ind_name,
                        "parameter": p,
                        "label": f"{ind_name} Ichimoku {p.capitalize()}",
                        "type": "int",
                        "current_value": ind.get(p, 9 if p == "tenkan" else (26 if p == "kijun" else 52)),
                        "path": f"indicators[{idx}].{p}",
                    })
            elif ind_type == "relative_strength":
                params.append({
                    "target_type": "indicator",
                    "target_name": ind_name,
                    "parameter": "length",
                    "label": f"{ind_name} RS Length",
                    "type": "int",
                    "current_value": ind.get("length", 55),
                    "path": f"indicators[{idx}].length",
                })

        # Position sizing
        pos_sizing = strategy.get("position_sizing") or {}
        params.append({
            "target_type": "position_sizing",
            "target_name": "position_sizing",
            "parameter": "quantity",
            "label": "Position Sizing Quantity",
            "type": "int",
            "current_value": pos_sizing.get("quantity", 100),
            "path": "position_sizing.quantity",
        })
        params.append({
            "target_type": "position_sizing",
            "target_name": "position_sizing",
            "parameter": "percent",
            "label": "Position Sizing Equity %",
            "type": "float",
            "current_value": pos_sizing.get("percent", 10.0),
            "path": "position_sizing.percent",
        })

        # Risk management
        risk_mgmt = strategy.get("risk_management") or {}
        params.append({
            "target_type": "risk_management",
            "target_name": "risk_management",
            "parameter": "stop_loss_pct",
            "label": "Stop Loss %",
            "type": "float",
            "current_value": risk_mgmt.get("stop_loss_pct"),
            "path": "risk_management.stop_loss_pct",
        })
        params.append({
            "target_type": "risk_management",
            "target_name": "risk_management",
            "parameter": "take_profit_pct",
            "label": "Take Profit %",
            "type": "float",
            "current_value": risk_mgmt.get("take_profit_pct"),
            "path": "risk_management.take_profit_pct",
        })

        return params

    @staticmethod
    def validate_strategy_definition(strategy_dict: dict) -> dict:
        """Declarative validation of strategy config and AST rules without Python eval."""
        errors = []
        try:
            strategy_obj = load_strategy_from_dict(strategy_dict)
        except Exception as exc:
            return {"valid": False, "errors": [f"Invalid strategy schema: {str(exc)}"]}

        try:
            # Check rule AST safety
            indicator_names = {ind.name for ind in strategy_obj.indicators}
            indicator_names.update(["open", "high", "low", "close", "volume"])
            StrategyRuleEvaluator.validate_strategy_rules(strategy_obj, indicator_names)
        except RuleEvaluationError as exc:
            errors.append(f"Rule validation error: {str(exc)}")
        except Exception as exc:
            errors.append(f"Unexpected rule error: {str(exc)}")

        return {
            "valid": len(errors) == 0,
            "strategy_name": strategy_obj.name,
            "version": strategy_obj.version,
            "indicators": [ind.model_dump() for ind in strategy_obj.indicators],
            "errors": errors,
        }

    async def run_parameter_sweep(self, db: Session, config: dict) -> dict:
        sweep = config.get("sweep") or []
        if not sweep:
            return {
                "status": "failed",
                "error_code": "INVALID_SWEEP",
                "message": "At least one sweep parameter is required.",
                "variants": [],
            }

        start_date = config.get("start_date")
        end_date = config.get("end_date")
        oos_start_date = config.get("oos_start_date")
        oos_end_date = config.get("oos_end_date")

        self._validate_date_ranges(start_date, end_date, oos_start_date, oos_end_date)

        max_variants = int(config.get("max_variants", 30))
        if max_variants < 1:
            max_variants = 1
        elif max_variants > 50:
            max_variants = 50

        sweep_id = config.get("sweep_id") or str(uuid.uuid4())
        SweepCancellationManager.register_sweep(sweep_id)

        try:
            variants = self._build_variants(config["strategy"], sweep, max_variants)
            results = []
            is_cancelled = False

            for variant in variants:
                if SweepCancellationManager.is_cancelled(sweep_id):
                    is_cancelled = True
                    break

                # 1. In-Sample Backtest
                is_run_config = {
                    "symbol": config.get("symbol"),
                    "symbols": config.get("symbols"),
                    "start_date": start_date,
                    "end_date": end_date,
                    "initial_cash": config.get("initial_cash", 100000000),
                    "benchmark_symbol": config.get("benchmark_symbol", "VNINDEX"),
                    "strategy": variant["strategy"],
                }
                is_response = await self.backtest_service.run_backtest(db, is_run_config)

                # 2. Out-of-Sample Backtest if configured
                oos_response = None
                if oos_start_date and oos_end_date:
                    if SweepCancellationManager.is_cancelled(sweep_id):
                        is_cancelled = True
                        break
                    oos_run_config = {
                        "symbol": config.get("symbol"),
                        "symbols": config.get("symbols"),
                        "start_date": oos_start_date,
                        "end_date": oos_end_date,
                        "initial_cash": config.get("initial_cash", 100000000),
                        "benchmark_symbol": config.get("benchmark_symbol", "VNINDEX"),
                        "strategy": variant["strategy"],
                    }
                    oos_response = await self.backtest_service.run_backtest(db, oos_run_config)

                metrics = self._extract_metrics(is_response, oos_response)
                results.append({
                    "label": variant["label"],
                    "parameters": variant["parameters"],
                    "response": self._compact_response(is_response),
                    "oos_response": self._compact_response(oos_response) if oos_response else None,
                    "metrics": metrics,
                })

            sorted_variants = self._sort_variants(results)

            return {
                "status": "cancelled" if is_cancelled else "succeeded",
                "sweep_id": sweep_id,
                "cancelled": is_cancelled,
                "total_variants": len(results),
                "truncated": len(results) >= max_variants,
                "ranking_metric": "net_pnl",
                "in_sample_period": {"start_date": start_date, "end_date": end_date},
                "out_of_sample_period": (
                    {"start_date": oos_start_date, "end_date": oos_end_date}
                    if oos_start_date and oos_end_date else None
                ),
                "variants": sorted_variants,
                "execution_assumptions": results[0]["response"].get("execution_assumptions") if results else None,
            }
        finally:
            SweepCancellationManager.cleanup_sweep(sweep_id)

    def _validate_date_ranges(
        self,
        start_date: Optional[str],
        end_date: Optional[str],
        oos_start_date: Optional[str],
        oos_end_date: Optional[str],
    ) -> None:
        if not start_date or not end_date:
            raise ValueError("start_date and end_date are required.")

        try:
            is_start = datetime.fromisoformat(start_date).date() if isinstance(start_date, str) else start_date
            is_end = datetime.fromisoformat(end_date).date() if isinstance(end_date, str) else end_date
        except Exception as exc:
            raise ValueError(f"Invalid in-sample date format: {exc}")

        if is_start >= is_end:
            raise ValueError(f"In-sample start date ({start_date}) must be before end date ({end_date}).")

        if bool(oos_start_date) != bool(oos_end_date):
            raise ValueError("Both out-of-sample start and end dates must be provided if out-of-sample testing is used.")

        if oos_start_date and oos_end_date:
            try:
                oos_start = datetime.fromisoformat(oos_start_date).date() if isinstance(oos_start_date, str) else oos_start_date
                oos_end = datetime.fromisoformat(oos_end_date).date() if isinstance(oos_end_date, str) else oos_end_date
            except Exception as exc:
                raise ValueError(f"Invalid out-of-sample date format: {exc}")

            if oos_start >= oos_end:
                raise ValueError(f"Out-of-sample start date ({oos_start_date}) must be before end date ({oos_end_date}).")

            # Non-overlapping check: In-sample and out-of-sample date ranges must not overlap
            if not (is_end <= oos_start or oos_end <= is_start):
                raise ValueError(
                    f"In-sample [{start_date} to {end_date}] and out-of-sample [{oos_start_date} to {oos_end_date}] date ranges must not overlap."
                )

    def _build_variants(self, strategy: dict, sweep: list[dict], max_variants: int) -> list[dict]:
        resolved_sweep = []
        for item in sweep:
            path = item.get("path")
            if not path and item.get("target_type") and item.get("parameter"):
                path = self._resolve_sweep_path(strategy, item)
            values = item.get("values") or []
            if not path or not values:
                raise ValueError("Each sweep item must include path/target and values.")
            resolved_sweep.append({
                "path": path,
                "values": values,
                "target_type": item.get("target_type"),
                "target_name": item.get("target_name"),
                "parameter": item.get("parameter"),
            })

        paths = [item["path"] for item in resolved_sweep]
        value_lists = [item["values"] for item in resolved_sweep]

        variants = []
        for values in product(*value_lists):
            variant_strategy = deepcopy(strategy)
            parameters = {}
            for path, value in zip(paths, values):
                self._set_path_value(variant_strategy, path, value)
                parameters[path] = value

            # Build readable label
            label_parts = []
            for item, value in zip(resolved_sweep, values):
                if item.get("target_name") and item.get("parameter"):
                    label_parts.append(f"{item['target_name']}.{item['parameter']}={value}")
                else:
                    label_parts.append(f"{item['path']}={value}")

            variants.append({
                "label": ", ".join(label_parts),
                "parameters": parameters,
                "strategy": variant_strategy,
            })
            if len(variants) >= max_variants:
                break
        return variants

    def _resolve_sweep_path(self, strategy: dict, item: dict) -> str:
        target_type = item.get("target_type")
        target_name = item.get("target_name")
        param = item.get("parameter")
        if not param:
            raise ValueError("Parameter name is required for typed sweep definition.")

        if target_type == "indicator":
            indicators = strategy.get("indicators", [])
            for idx, ind in enumerate(indicators):
                if ind.get("name") == target_name:
                    return f"indicators[{idx}].{param}"
            if target_name and str(target_name).isdigit():
                idx = int(target_name)
                if 0 <= idx < len(indicators):
                    return f"indicators[{idx}].{param}"
            raise ValueError(f"Indicator '{target_name}' not found in strategy.")

        if target_type == "position_sizing":
            return f"position_sizing.{param}"

        if target_type == "risk_management":
            return f"risk_management.{param}"

        raise ValueError(f"Unsupported target_type: {target_type}")

    def _set_path_value(self, target: dict, path: str, value) -> None:
        current = target
        parts = path.split(".")
        for part in parts[:-1]:
            current = self._descend(current, part)
        final_key = parts[-1]
        if "[" in final_key:
            parent = self._descend(current, final_key, descend_to_parent=True)
            key, index = self._parse_indexed_key(final_key)
            parent[key][index] = value
        else:
            current[final_key] = value

    def _descend(self, current, part: str, descend_to_parent: bool = False):
        key, index = self._parse_indexed_key(part)
        if index is None:
            return current if descend_to_parent else current[key]
        return current if descend_to_parent else current[key][index]

    def _parse_indexed_key(self, part: str) -> tuple[str, int | None]:
        match = re.fullmatch(r"([A-Za-z0-9_]+)(?:\[(\d+)])?", part)
        if not match:
            raise ValueError(f"Invalid parameter path segment: {part}")
        key = match.group(1)
        index = int(match.group(2)) if match.group(2) is not None else None
        return key, index

    def _extract_metrics(self, response: dict, oos_response: Optional[dict] = None) -> dict:
        analytics = response.get("analytics") or {}
        summary = response.get("summary") or {}
        metric_results = analytics.get("metrics") or {}
        trade_count = summary.get("total_trades", analytics.get("total_trades", 0))
        net_pnl = summary.get("total_net_pnl", analytics.get("total_net_pnl", 0.0))
        win_rate = summary.get("win_rate", analytics.get("win_rate", 0.0))
        profit_factor = analytics.get("profit_factor")
        expectancy = analytics.get("expectancy")
        ranking_metric = metric_results.get("total_net_pnl") or {}

        # Minimum 5 trades required to be eligible for winning ranking (PRO-STRAT-05)
        has_min_trades = trade_count >= 5
        is_valid_metric = ranking_metric.get("status") == "valid" if ranking_metric else False
        is_succeeded = response.get("status") == "succeeded"

        ranking_eligible = is_succeeded and has_min_trades and is_valid_metric
        if not is_succeeded:
            ranking_reason = "Variant did not succeed."
        elif not has_min_trades:
            ranking_reason = f"Insufficient sample size ({trade_count} trades < 5 required)."
        elif not is_valid_metric:
            ranking_reason = f"Metric total_net_pnl status is '{ranking_metric.get('status', 'invalid')}'."
        else:
            ranking_reason = None

        metrics = {
            "status": response.get("status", "succeeded"),
            "total_trades": trade_count,
            "win_rate": win_rate,
            "net_pnl": net_pnl,
            "profit_factor": profit_factor,
            "expectancy": expectancy,
            "metric_results": metric_results,
            "ranking_eligible": ranking_eligible,
            "ranking_reason": ranking_reason,
        }

        if oos_response:
            oos_analytics = oos_response.get("analytics") or {}
            oos_summary = oos_response.get("summary") or {}
            oos_metric_results = oos_analytics.get("metrics") or {}
            oos_trades = oos_summary.get("total_trades", oos_analytics.get("total_trades", 0))
            oos_net_pnl = oos_summary.get("total_net_pnl", oos_analytics.get("total_net_pnl", 0.0))
            oos_win_rate = oos_summary.get("win_rate", oos_analytics.get("win_rate", 0.0))
            oos_pf = oos_analytics.get("profit_factor")
            oos_expectancy = oos_analytics.get("expectancy")

            # Stability ratio calculation
            stability_ratio = None
            if net_pnl > 0 and oos_net_pnl > 0:
                stability_ratio = round(min(net_pnl, oos_net_pnl) / max(net_pnl, oos_net_pnl), 4)
            elif net_pnl > 0 and oos_net_pnl <= 0:
                stability_ratio = round(oos_net_pnl / net_pnl, 4)
            elif net_pnl != 0:
                stability_ratio = round(oos_net_pnl / abs(net_pnl), 4)

            pf_degradation = None
            if profit_factor is not None and oos_pf is not None and profit_factor > 0:
                pf_degradation = round(oos_pf / profit_factor, 4)

            # Robustness badge classification
            if trade_count < 5:
                robustness_badge = "Low Sample"
                robustness_score = 0.0
            elif net_pnl <= 0:
                robustness_badge = "Unprofitable"
                robustness_score = 0.0
            elif oos_net_pnl > 0 and (oos_pf is None or oos_pf >= 1.0):
                robustness_badge = "Robust"
                robustness_score = round((stability_ratio if stability_ratio is not None else 0.5) * 100.0, 1)
            elif oos_net_pnl <= 0:
                robustness_badge = "Overfitted"
                robustness_score = 10.0
            else:
                robustness_badge = "Degraded"
                robustness_score = 30.0

            metrics["out_of_sample"] = {
                "status": oos_response.get("status", "succeeded"),
                "total_trades": oos_trades,
                "win_rate": oos_win_rate,
                "net_pnl": oos_net_pnl,
                "profit_factor": oos_pf,
                "expectancy": oos_expectancy,
                "metric_results": oos_metric_results,
            }
            metrics["robustness"] = {
                "badge": robustness_badge,
                "score": robustness_score,
                "stability_ratio": stability_ratio,
                "profit_factor_degradation": pf_degradation,
                "sample_size_is": trade_count,
                "sample_size_oos": oos_trades,
            }
            metrics["robustness_score"] = robustness_score
            metrics["robustness_badge"] = robustness_badge
        else:
            if trade_count < 5:
                robustness_badge = "Low Sample"
                robustness_score = 0.0
            elif net_pnl <= 0:
                robustness_badge = "Unprofitable"
                robustness_score = 0.0
            else:
                robustness_badge = "Unvalidated"
                robustness_score = 50.0

            metrics["robustness"] = {
                "badge": robustness_badge,
                "score": robustness_score,
                "stability_ratio": None,
                "profit_factor_degradation": None,
                "sample_size_is": trade_count,
                "sample_size_oos": None,
            }
            metrics["robustness_score"] = robustness_score
            metrics["robustness_badge"] = robustness_badge

        return metrics

    def _sort_variants(self, variants: list[dict]) -> list[dict]:
        return sorted(
            variants,
            key=lambda row: (
                1 if row["metrics"]["ranking_eligible"] else 0,
                row["metrics"].get("robustness_score", 0.0) if row["metrics"]["ranking_eligible"] else float("-inf"),
                row["metrics"]["net_pnl"] if row["metrics"]["ranking_eligible"] else float("-inf"),
            ),
            reverse=True,
        )

    def _compact_response(self, response: Optional[dict]) -> Optional[dict]:
        if not response:
            return None
        analytics = response.get("analytics") or {}
        summary = response.get("summary") or {}
        compact = {
            "status": response.get("status", "succeeded"),
            "symbol": response.get("symbol"),
            "symbols": response.get("symbols"),
            "total_candles": response.get("total_candles"),
            "analytics": {
                "total_trades": summary.get("total_trades", analytics.get("total_trades", 0)),
                "win_rate": summary.get("win_rate", analytics.get("win_rate", 0.0)),
                "total_net_pnl": summary.get("total_net_pnl", analytics.get("total_net_pnl", 0.0)),
                "profit_factor": analytics.get("profit_factor"),
                "expectancy": analytics.get("expectancy"),
                "metrics": analytics.get("metrics", {}),
            },
            "data_coverage": response.get("data_coverage"),
            "execution_assumptions": response.get("execution_assumptions"),
            "run_manifest": response.get("run_manifest"),
        }
        if response.get("message"):
            compact["message"] = response["message"]
        return compact
