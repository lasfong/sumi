from typing import Any

import pandas as pd

from app.domain.engine.indicator_engine import IndicatorEngine
from app.domain.signals.models import CandleBar
from app.domain.signals.registry import SignalRegistry


SOURCE_COLUMNS = {"timestamp", "open", "high", "low", "close", "volume"}


class StrategyIndicatorAdapter:
    """Maps strategy indicator configs onto the shared IndicatorEngine and SignalRegistry output."""

    @staticmethod
    def compute(df: pd.DataFrame, indicators_config) -> dict[str, Any]:
        values: dict[str, Any] = {}
        candle_bars: list[CandleBar] | None = None
        for indicator in indicators_config:
            sig_name = (
                SignalRegistry.resolve_name_from_alias(indicator.name)
                or (indicator.name if SignalRegistry.get_definition(indicator.name) else None)
                or SignalRegistry.resolve_name_from_alias(indicator.name.replace("_", "__", 1))
            )
            if sig_name is not None:
                if candle_bars is None:
                    candle_bars = [
                        CandleBar(
                            index=idx,
                            timestamp=row["timestamp"].isoformat()
                            if hasattr(row["timestamp"], "isoformat")
                            else str(row["timestamp"]),
                            open=float(row["open"]),
                            high=float(row["high"]),
                            low=float(row["low"]),
                            close=float(row["close"]),
                            volume=float(row["volume"]) if row.get("volume") is not None else None,
                        )
                        for idx, row in enumerate(df.to_dict("records"))
                    ]
                resolved_params = SignalRegistry.resolve_and_validate_params(sig_name, None)
                calc_result = SignalRegistry.calculate(
                    name=sig_name,
                    candles=candle_bars,
                    params=resolved_params,
                    session_id=0,
                    observed_index=max(0, len(candle_bars) - 1),
                )
                values[indicator.name] = [
                    pt.value if pt.quality.value == "VALID" else None
                    for pt in calc_result.points
                ]
                continue

            raw_type = indicator.type
            inferred_length = getattr(indicator, "length", None)
            if raw_type == "signal" and "_" in indicator.name:
                prefix, suffix = indicator.name.rsplit("_", 1)
                if suffix.isdigit():
                    raw_type = prefix
                    if inferred_length is None:
                        inferred_length = int(suffix)

            indicator_id = IndicatorEngine.normalize_indicator_id(raw_type)
            params = StrategyIndicatorAdapter._params_for(indicator)
            if inferred_length is not None and "length" not in params:
                params["length"] = inferred_length
            result_df = IndicatorEngine.compute(df, indicator_id, **params)
            output_columns = [
                column for column in result_df.columns
                if column not in SOURCE_COLUMNS and column not in df.columns
            ]
            values.update(
                StrategyIndicatorAdapter._map_columns(
                    result_df=result_df,
                    output_columns=output_columns,
                    indicator_name=indicator.name,
                    indicator_id=indicator_id,
                )
            )
        for col in ("open", "high", "low", "close", "volume"):
            if col in df.columns:
                values[col] = df[col].values
        return values

    @staticmethod
    def _params_for(indicator) -> dict[str, Any]:
        params: dict[str, Any] = {}
        for name in (
            "length", "fast", "slow", "signal", "std", "k", "d", "smooth_k", "benchmark",
            "scalar", "af0", "af", "max_af", "multiplier", "tenkan", "kijun", "senkou",
        ):
            value = getattr(indicator, name, None)
            if value is not None:
                params[name] = value
        return params

    @staticmethod
    def _map_columns(
        result_df: pd.DataFrame,
        output_columns: list[str],
        indicator_name: str,
        indicator_id: str,
    ) -> dict[str, Any]:
        if indicator_id == "macd":
            return StrategyIndicatorAdapter._map_macd_columns(result_df, output_columns, indicator_name)
        if indicator_id == "ichimoku":
            return StrategyIndicatorAdapter._map_ichimoku_columns(result_df, output_columns, indicator_name)

        if len(output_columns) == 1:
            return {indicator_name: result_df[output_columns[0]].values}

        return {
            f"{indicator_name}_{column.lower()}": result_df[column].values
            for column in output_columns
        }

    @staticmethod
    def _map_macd_columns(
        result_df: pd.DataFrame,
        output_columns: list[str],
        indicator_name: str,
    ) -> dict[str, Any]:
        mapped: dict[str, Any] = {}
        for column in output_columns:
            key = column.lower()
            if key.startswith("macdh"):
                mapped[f"{indicator_name}_hist"] = result_df[column].values
            elif key.startswith("macds"):
                mapped[f"{indicator_name}_signal"] = result_df[column].values
            elif key.startswith("macd"):
                mapped[f"{indicator_name}_line"] = result_df[column].values
        return mapped

    @staticmethod
    def _map_ichimoku_columns(
        result_df: pd.DataFrame,
        output_columns: list[str],
        indicator_name: str,
    ) -> dict[str, Any]:
        mapped: dict[str, Any] = {}
        for column in output_columns:
            key = column.upper()
            vals = result_df[column].values
            mapped[f"{indicator_name}_{column.lower()}"] = vals
            if key.startswith("ITS"):
                mapped[f"{indicator_name}_tenkan"] = vals
            elif key.startswith("IKS"):
                mapped[f"{indicator_name}_kijun"] = vals
            elif key.startswith("ISA"):
                mapped[f"{indicator_name}_span_a"] = vals
            elif key.startswith("ISB"):
                mapped[f"{indicator_name}_span_b"] = vals
            elif key.startswith("ICS"):
                mapped[f"{indicator_name}_chikou"] = vals
        return mapped
