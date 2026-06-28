from copy import deepcopy
from itertools import product
import re
from sqlalchemy.orm import Session

from app.services.backtest_service import BacktestService


class StrategyLabService:
    def __init__(self):
        self.backtest_service = BacktestService()

    async def run_parameter_sweep(self, db: Session, config: dict) -> dict:
        sweep = config.get("sweep") or []
        if not sweep:
            return {
                "status": "failed",
                "error_code": "INVALID_SWEEP",
                "message": "At least one sweep parameter is required.",
                "variants": [],
            }

        max_variants = int(config.get("max_variants", 30))
        variants = self._build_variants(config["strategy"], sweep, max_variants)
        results = []

        for variant in variants:
            run_config = {
                "symbol": config.get("symbol"),
                "symbols": config.get("symbols"),
                "start_date": config["start_date"],
                "end_date": config["end_date"],
                "initial_cash": config.get("initial_cash", 100000000),
                "benchmark_symbol": config.get("benchmark_symbol", "VNINDEX"),
                "strategy": variant["strategy"],
            }
            response = await self.backtest_service.run_backtest(db, run_config)
            results.append({
                "label": variant["label"],
                "parameters": variant["parameters"],
                "response": response,
                "metrics": self._extract_metrics(response),
            })

        return {
            "status": "succeeded",
            "total_variants": len(results),
            "truncated": len(results) >= max_variants,
            "variants": sorted(results, key=lambda row: row["metrics"]["net_pnl"], reverse=True),
        }

    def _build_variants(self, strategy: dict, sweep: list[dict], max_variants: int) -> list[dict]:
        paths = [item.get("path") for item in sweep]
        value_lists = [item.get("values") or [] for item in sweep]
        if any(not path for path in paths) or any(not values for values in value_lists):
            raise ValueError("Each sweep item must include path and values.")

        variants = []
        for values in product(*value_lists):
            variant_strategy = deepcopy(strategy)
            parameters = {}
            for path, value in zip(paths, values):
                self._set_path_value(variant_strategy, path, value)
                parameters[path] = value
            variants.append({
                "label": ", ".join(f"{path}={value}" for path, value in parameters.items()),
                "parameters": parameters,
                "strategy": variant_strategy,
            })
            if len(variants) >= max_variants:
                break
        return variants

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

    def _extract_metrics(self, response: dict) -> dict:
        analytics = response.get("analytics") or {}
        summary = response.get("summary") or {}
        return {
            "status": response.get("status", "succeeded"),
            "total_trades": summary.get("total_trades", analytics.get("total_trades", 0)),
            "win_rate": summary.get("win_rate", analytics.get("win_rate", 0.0)),
            "net_pnl": summary.get("total_net_pnl", analytics.get("total_net_pnl", 0.0)),
            "profit_factor": analytics.get("profit_factor"),
            "expectancy": analytics.get("expectancy"),
        }
