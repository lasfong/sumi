import pandas as pd
from sqlalchemy.orm import Session

from app.domain.regime.regime_classifier import RegimeClassifier
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle
from app.services.backtest_service import BacktestService


class ScannerService:
    def __init__(self):
        self.backtest_service = BacktestService()

    def run_scan(self, db: Session, config: dict) -> dict:
        symbols = config.get("symbols") or []
        symbols = [symbol.strip().upper() for symbol in symbols if symbol and symbol.strip()]
        if not symbols:
            return {
                "status": "failed",
                "error_code": "INVALID_SYMBOLS",
                "message": "At least one symbol is required.",
                "results": [],
                "warnings": [],
            }

        strategy = load_strategy_from_dict(config["strategy"])
        start_date = config["start_date"]
        end_date = config["end_date"]
        benchmark_symbol = config.get("benchmark_symbol", "VNINDEX")
        max_results = int(config.get("max_results", 500))
        warnings = []
        results = []
        regime_map = self._build_regime_map(db, benchmark_symbol, start_date, end_date)

        for symbol in symbols:
            candles = self._load_candles(db, symbol, start_date, end_date)
            if not candles:
                warnings.append(f"No candles found for {symbol}")
                continue

            df = self._to_dataframe(candles)
            indicator_values = self.backtest_service._compute_indicators(df, strategy.indicators)
            try:
                self.backtest_service._validate_strategy_rules(strategy, set(indicator_values.keys()))
            except RuleEvaluationError as exc:
                return {
                    "status": "failed",
                    "error_code": "INVALID_RULE",
                    "message": str(exc),
                    "results": [],
                    "warnings": warnings,
                }

            for index in range(1, len(df)):
                current = self.backtest_service._get_indicator_snapshot(indicator_values, index)
                previous = self.backtest_service._get_indicator_snapshot(indicator_values, index - 1)
                if self.backtest_service._evaluate_rules(strategy.entry_rules, current, previous):
                    timestamp = df.iloc[index]["timestamp"]
                    results.append({
                        "symbol": symbol,
                        "timestamp": timestamp.isoformat() if hasattr(timestamp, "isoformat") else str(timestamp),
                        "signal_type": "entry",
                        "strategy": strategy.name,
                        "price": float(df.iloc[index]["close"]),
                        "regime": self._lookup_regime(timestamp, regime_map),
                    })
                    if len(results) >= max_results:
                        return self._build_response(results, warnings, max_results)

        return self._build_response(results, warnings, max_results)

    def _build_response(self, results: list[dict], warnings: list[str], max_results: int) -> dict:
        return {
            "status": "succeeded",
            "total_results": len(results),
            "truncated": len(results) >= max_results,
            "results": results,
            "warnings": warnings,
        }

    def _load_candles(self, db: Session, symbol: str, start_date: str, end_date: str) -> list[Candle]:
        return db.query(Candle).filter(
            Candle.symbol == symbol,
            Candle.timestamp >= start_date,
            Candle.timestamp <= end_date,
        ).order_by(Candle.timestamp).all()

    def _build_regime_map(self, db: Session, benchmark_symbol: str | None, start_date: str, end_date: str) -> dict:
        if not benchmark_symbol:
            return {}

        benchmark_candles = self._load_candles(db, benchmark_symbol, start_date, end_date)
        return RegimeClassifier.build_regime_map(benchmark_candles) if benchmark_candles else {}

    def _lookup_regime(self, timestamp, regime_map: dict) -> str:
        if not regime_map:
            return "Unknown"
        signal_date = timestamp.date() if hasattr(timestamp, "date") else timestamp
        return regime_map.get(signal_date, "Unknown")

    def _to_dataframe(self, candles: list[Candle]) -> pd.DataFrame:
        return pd.DataFrame([{
            "timestamp": candle.timestamp,
            "open": candle.open,
            "high": candle.high,
            "low": candle.low,
            "close": candle.close,
            "volume": candle.volume,
        } for candle in candles])
