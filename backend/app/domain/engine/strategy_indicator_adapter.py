from typing import Any

import pandas as pd

from app.domain.engine.indicator_engine import IndicatorEngine


SOURCE_COLUMNS = {"timestamp", "open", "high", "low", "close", "volume"}


class StrategyIndicatorAdapter:
    """Maps strategy indicator configs onto the shared IndicatorEngine output."""

    @staticmethod
    def compute(df: pd.DataFrame, indicators_config) -> dict[str, Any]:
        values: dict[str, Any] = {}
        for indicator in indicators_config:
            indicator_id = IndicatorEngine.normalize_indicator_id(indicator.type)
            params = StrategyIndicatorAdapter._params_for(indicator)
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
