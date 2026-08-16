from dataclasses import asdict, dataclass
from datetime import date, datetime
from typing import Any, Optional

import numpy as np
import pandas as pd
import pandas_ta  # noqa: F401 - registers the pandas .ta accessor


@dataclass(frozen=True)
class IndicatorParam:
    name: str
    type: str
    default: Optional[float | int | str | bool] = None
    minimum: Optional[float] = None
    maximum: Optional[float] = None


@dataclass(frozen=True)
class IndicatorDefinition:
    id: str
    label: str
    category: str
    pane: str
    method: Optional[str]
    params: tuple[IndicatorParam, ...]
    aliases: tuple[str, ...] = ()
    description: str = ""

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["params"] = [asdict(param) for param in self.params]
        return data


COMMON_PARAMS = (
    IndicatorParam("offset", "int", 0),
)


INDICATOR_REGISTRY: dict[str, IndicatorDefinition] = {
    "sma": IndicatorDefinition(
        id="sma",
        label="Simple Moving Average",
        category="Trend",
        pane="main",
        method="sma",
        params=(IndicatorParam("length", "int", 20, 1, 500), *COMMON_PARAMS),
        description="Average closing price over a fixed lookback window.",
    ),
    "ema": IndicatorDefinition(
        id="ema",
        label="Exponential Moving Average",
        category="Trend",
        pane="main",
        method="ema",
        params=(IndicatorParam("length", "int", 20, 1, 500), *COMMON_PARAMS),
        description="Moving average that weights recent closes more heavily.",
    ),
    "macd": IndicatorDefinition(
        id="macd",
        label="MACD",
        category="Trend",
        pane="oscillator",
        method="macd",
        params=(
            IndicatorParam("fast", "int", 12, 1, 200),
            IndicatorParam("slow", "int", 26, 2, 400),
            IndicatorParam("signal", "int", 9, 1, 200),
            *COMMON_PARAMS,
        ),
        description="Trend momentum indicator based on fast/slow EMA spread.",
    ),
    "rsi": IndicatorDefinition(
        id="rsi",
        label="Relative Strength Index",
        category="Oscillators",
        pane="oscillator",
        method="rsi",
        params=(IndicatorParam("length", "int", 14, 1, 200), *COMMON_PARAMS),
        description="Momentum oscillator scaled from 0 to 100.",
    ),
    "bbands": IndicatorDefinition(
        id="bbands",
        label="Bollinger Bands",
        category="Volatility",
        pane="main",
        method="bbands",
        params=(
            IndicatorParam("length", "int", 20, 1, 500),
            IndicatorParam("std", "float", 2.0, 0.1, 10),
            *COMMON_PARAMS,
        ),
        aliases=("bollinger", "bollinger_bands"),
        description="Volatility bands around a moving average.",
    ),
    "atr": IndicatorDefinition(
        id="atr",
        label="Average True Range",
        category="Volatility",
        pane="oscillator",
        method="atr",
        params=(IndicatorParam("length", "int", 14, 1, 200), *COMMON_PARAMS),
        description="Average range-based volatility.",
    ),
    "adx": IndicatorDefinition(
        id="adx",
        label="Average Directional Index",
        category="Trend",
        pane="oscillator",
        method="adx",
        params=(IndicatorParam("length", "int", 14, 1, 200), *COMMON_PARAMS),
        description="Trend strength indicator using directional movement.",
    ),
    "ichimoku": IndicatorDefinition(
        id="ichimoku",
        label="Ichimoku Cloud",
        category="Trend",
        pane="main",
        method="ichimoku",
        params=(
            IndicatorParam("tenkan", "int", 9, 1, 200),
            IndicatorParam("kijun", "int", 26, 1, 400),
            IndicatorParam("senkou", "int", 52, 1, 800),
            *COMMON_PARAMS,
        ),
        description="Trend, support/resistance and momentum cloud system.",
    ),
    "stoch": IndicatorDefinition(
        id="stoch",
        label="Stochastic Oscillator",
        category="Oscillators",
        pane="oscillator",
        method="stoch",
        params=(
            IndicatorParam("k", "int", 14, 1, 200),
            IndicatorParam("d", "int", 3, 1, 50),
            IndicatorParam("smooth_k", "int", 3, 1, 50),
            *COMMON_PARAMS,
        ),
        aliases=("stochastic",),
        description="Compares close to the recent high-low range.",
    ),
    "volume_sma": IndicatorDefinition(
        id="volume_sma",
        label="Volume Moving Average",
        category="Volume",
        pane="oscillator",
        method=None,
        params=(IndicatorParam("length", "int", 20, 1, 500),),
        aliases=("vma", "volume_ma"),
        description="Simple moving average of volume.",
    ),
    "psar": IndicatorDefinition(
        id="psar",
        label="Parabolic SAR",
        category="Trend",
        pane="main",
        method="psar",
        params=(
            IndicatorParam("af0", "float", 0.02, 0.001, 1),
            IndicatorParam("af", "float", 0.02, 0.001, 1),
            IndicatorParam("max_af", "float", 0.2, 0.001, 1),
            *COMMON_PARAMS,
        ),
        aliases=("sar", "parabolic_sar"),
        description="Trailing stop and trend-following indicator.",
    ),
    "supertrend": IndicatorDefinition(
        id="supertrend",
        label="SuperTrend",
        category="Trend",
        pane="main",
        method="supertrend",
        params=(
            IndicatorParam("length", "int", 7, 1, 200),
            IndicatorParam("multiplier", "float", 3.0, 0.1, 20),
            *COMMON_PARAMS,
        ),
        aliases=("st",),
        description="ATR-based trend following overlay.",
    ),
    "cci": IndicatorDefinition(
        id="cci",
        label="Commodity Channel Index",
        category="Oscillators",
        pane="oscillator",
        method="cci",
        params=(IndicatorParam("length", "int", 20, 1, 200), *COMMON_PARAMS),
        description="Momentum oscillator around typical price.",
    ),
    "mfi": IndicatorDefinition(
        id="mfi",
        label="Money Flow Index",
        category="Oscillators",
        pane="oscillator",
        method="mfi",
        params=(IndicatorParam("length", "int", 14, 1, 200), *COMMON_PARAMS),
        description="Volume-weighted momentum oscillator.",
    ),
    "kc": IndicatorDefinition(
        id="kc",
        label="Keltner Channels",
        category="Volatility",
        pane="main",
        method="kc",
        params=(
            IndicatorParam("length", "int", 20, 1, 500),
            IndicatorParam("scalar", "float", 2.0, 0.1, 20),
            *COMMON_PARAMS,
        ),
        aliases=("keltner", "keltner_channels"),
        description="ATR-based volatility channel.",
    ),
    "relative_strength": IndicatorDefinition(
        id="relative_strength",
        label="Relative Strength (vs VNINDEX)",
        category="Oscillators",
        pane="oscillator",
        method=None,
        params=(
            IndicatorParam("length", "int", 20, 1, 500),
            IndicatorParam("benchmark", "str", "VNINDEX"),
            *COMMON_PARAMS,
        ),
        aliases=("rs", "rs_vnindex", "rel_strength"),
        description="Ratio of symbol price performance to VNINDEX benchmark over a lookback window.",
    ),
}

ALIASES = {
    alias: indicator_id
    for indicator_id, definition in INDICATOR_REGISTRY.items()
    for alias in definition.aliases
}


class IndicatorEngine:
    """
    Registry-backed technical indicator engine.

    Only indicators declared in INDICATOR_REGISTRY can be computed. This keeps
    replay/backtest behavior explicit while still delegating formulas to
    pandas-ta where appropriate.
    """

    @staticmethod
    def list_definitions() -> list[dict[str, Any]]:
        return [definition.to_dict() for definition in INDICATOR_REGISTRY.values()]

    @staticmethod
    def get_definition(indicator: str) -> IndicatorDefinition:
        indicator_id = IndicatorEngine.normalize_indicator_id(indicator)
        return INDICATOR_REGISTRY[indicator_id]

    @staticmethod
    def normalize_indicator_id(indicator: str) -> str:
        indicator_id = indicator.lower().strip()
        indicator_id = ALIASES.get(indicator_id, indicator_id)
        if indicator_id not in INDICATOR_REGISTRY:
            raise ValueError(f"Indicator '{indicator}' is not supported.")
        return indicator_id

    @staticmethod
    def compute(df: pd.DataFrame, indicator: str, **kwargs) -> pd.DataFrame:
        """
        Compute a registered indicator and append its output columns.
        """
        definition = IndicatorEngine.get_definition(indicator)
        params = IndicatorEngine._normalize_params(definition, kwargs)
        df_copy = df.copy()

        if definition.id == "volume_sma":
            length = int(params.get("length", 20))
            df_copy[f"VOLUME_SMA_{length}"] = df_copy["volume"].rolling(window=length).mean()
            return df_copy

        if definition.id == "cci":
            return IndicatorEngine._compute_cci(df_copy, params)

        if definition.id == "relative_strength":
            benchmark_df = kwargs.get("benchmark_df")
            return IndicatorEngine._compute_relative_strength(df_copy, params, benchmark_df=benchmark_df)

        if definition.method is None:
            raise ValueError(f"Indicator '{definition.id}' does not have a compute method.")

        try:
            ta_params = dict(params)
            if definition.id == "bbands":
                std = ta_params.pop("std", 2.0)
                ta_params["lower_std"] = std
                ta_params["upper_std"] = std
            result = getattr(df_copy.ta, definition.method)(**ta_params)
            IndicatorEngine._append_indicator_result(df_copy, result)
            return df_copy
        except AttributeError:
            raise ValueError(f"Indicator '{definition.id}' is not supported by pandas-ta.")
        except Exception as exc:
            raise RuntimeError(f"Error computing indicator '{definition.id}': {exc}")

    @staticmethod
    def _normalize_params(definition: IndicatorDefinition, kwargs: dict[str, Any]) -> dict[str, Any]:
        allowed_params = {param.name: param for param in definition.params}
        unknown_params = set(kwargs) - set(allowed_params) - {"benchmark_df"}
        if unknown_params:
            names = ", ".join(sorted(unknown_params))
            raise ValueError(f"Unsupported parameter(s) for indicator '{definition.id}': {names}")

        normalized = {}
        for name, param in allowed_params.items():
            value = kwargs.get(name, param.default)
            if value is None:
                continue
            normalized[name] = IndicatorEngine._coerce_param(definition.id, param, value)
        return normalized

    @staticmethod
    def _coerce_param(indicator_id: str, param: IndicatorParam, value: Any) -> Any:
        try:
            if param.type == "int":
                coerced = int(value)
            elif param.type == "float":
                coerced = float(value)
            elif param.type == "bool":
                coerced = bool(value)
            else:
                coerced = str(value)
        except (TypeError, ValueError):
            raise ValueError(f"Parameter '{param.name}' for indicator '{indicator_id}' must be {param.type}.")

        if isinstance(coerced, (int, float)):
            if param.minimum is not None and coerced < param.minimum:
                raise ValueError(f"Parameter '{param.name}' for indicator '{indicator_id}' must be >= {param.minimum}.")
            if param.maximum is not None and coerced > param.maximum:
                raise ValueError(f"Parameter '{param.name}' for indicator '{indicator_id}' must be <= {param.maximum}.")
        return coerced

    @staticmethod
    def _append_indicator_result(df_copy: pd.DataFrame, result: Any) -> None:
        if isinstance(result, tuple):
            for item in result:
                IndicatorEngine._append_indicator_result(df_copy, item)
            return

        if isinstance(result, pd.DataFrame):
            for col in result.columns:
                series = result[col]
                if col in df_copy.columns:
                    df_copy[col] = df_copy[col].combine_first(series)
                else:
                    df_copy[col] = series
            return

        if isinstance(result, pd.Series):
            if result.name in df_copy.columns:
                df_copy[result.name] = df_copy[result.name].combine_first(result)
            else:
                df_copy[result.name] = result

    @staticmethod
    def _compute_cci(df_copy: pd.DataFrame, params: dict[str, Any]) -> pd.DataFrame:
        """Compute canonical CCI while pandas-ta beta's formula is incorrect."""
        length = int(params.get("length", 20))
        offset = int(params.get("offset", 0))
        typical_price = (df_copy["high"] + df_copy["low"] + df_copy["close"]) / 3
        mean = typical_price.rolling(length).mean()
        mean_deviation = typical_price.rolling(length).apply(
            lambda values: abs(values - values.mean()).mean(),
            raw=False,
        )
        result = (typical_price - mean) / (0.015 * mean_deviation)
        if offset:
            result = result.shift(offset)
        df_copy[f"CCI_{length}_0.015"] = result
        return df_copy

    @staticmethod
    def _compute_relative_strength(
        df_copy: pd.DataFrame,
        params: dict[str, Any],
        benchmark_df: Optional[pd.DataFrame] = None,
    ) -> pd.DataFrame:
        """
        Compute Relative Strength vs benchmark over lookback window `length`.
        Performance ratio = (symbol_perf / benchmark_perf) * 100.
        """
        length = int(params.get("length", 20))
        offset = int(params.get("offset", 0))
        benchmark_symbol = str(params.get("benchmark", "VNINDEX")).upper()
        col_name = f"RS_{benchmark_symbol}_{length}"

        if benchmark_df is None or benchmark_df.empty or "close" not in benchmark_df.columns:
            df_copy[col_name] = pd.Series(index=df_copy.index, dtype="float64")
            return df_copy

        def _to_date_str(val: Any) -> str:
            if isinstance(val, (pd.Timestamp, datetime, date)):
                return val.strftime("%Y-%m-%d")
            return str(val)[:10]

        df_timestamps = df_copy.index if "timestamp" not in df_copy.columns else df_copy["timestamp"]
        bench_timestamps = benchmark_df.index if "timestamp" not in benchmark_df.columns else benchmark_df["timestamp"]

        df_date_keys = [_to_date_str(ts) for ts in df_timestamps]
        bench_date_keys = [_to_date_str(ts) for ts in bench_timestamps]

        bench_series = pd.Series(
            data=benchmark_df["close"].astype(float).values,
            index=bench_date_keys,
        )
        bench_series = bench_series[~bench_series.index.duplicated(keep="last")]

        aligned_bench_close = pd.Series([bench_series.get(d, np.nan) for d in df_date_keys], index=df_copy.index, dtype="float64")

        symbol_close = df_copy["close"].astype(float)
        symbol_perf = symbol_close / symbol_close.shift(length)
        bench_perf = aligned_bench_close / aligned_bench_close.shift(length)

        rs = (symbol_perf / bench_perf) * 100.0
        if offset:
            rs = rs.shift(offset)

        df_copy[col_name] = rs
        return df_copy
