from dataclasses import dataclass
from datetime import date, datetime
from typing import Iterable


@dataclass(frozen=True)
class RegimePoint:
    timestamp: date
    close: float
    regime: str
    trend_pct: float
    volatility_pct: float


class RegimeClassifier:
    """
    Deterministic benchmark regime classifier for research slicing.

    The classifier is intentionally simple for V2: it labels each benchmark bar
    from rolling trend and volatility so backtest results can be compared by
    bullish, sideways, bearish and volatile conditions.
    """

    DEFAULT_LOOKBACK = 20
    DEFAULT_BULLISH_THRESHOLD = 0.05
    DEFAULT_BEARISH_THRESHOLD = -0.05
    DEFAULT_SIDEWAYS_THRESHOLD = 0.03
    DEFAULT_VOLATILE_THRESHOLD = 0.025

    @classmethod
    def classify_candles(
        cls,
        candles: Iterable,
        lookback: int = DEFAULT_LOOKBACK,
        bullish_threshold: float = DEFAULT_BULLISH_THRESHOLD,
        bearish_threshold: float = DEFAULT_BEARISH_THRESHOLD,
        sideways_threshold: float = DEFAULT_SIDEWAYS_THRESHOLD,
        volatile_threshold: float = DEFAULT_VOLATILE_THRESHOLD,
    ) -> list[RegimePoint]:
        ordered = sorted(candles, key=lambda candle: candle.timestamp)
        if not ordered:
            return []

        closes = [float(candle.close) for candle in ordered]
        points = []

        for index, candle in enumerate(ordered):
            start = max(0, index - lookback)
            base_close = closes[start]
            current_close = closes[index]
            trend_pct = (current_close / base_close - 1) if base_close > 0 else 0.0
            window = closes[start:index + 1]
            volatility_pct = cls._average_absolute_return(window)

            regime = cls._label_regime(
                trend_pct=trend_pct,
                volatility_pct=volatility_pct,
                bullish_threshold=bullish_threshold,
                bearish_threshold=bearish_threshold,
                sideways_threshold=sideways_threshold,
                volatile_threshold=volatile_threshold,
            )

            points.append(RegimePoint(
                timestamp=cls._to_date(candle.timestamp),
                close=current_close,
                regime=regime,
                trend_pct=round(trend_pct, 6),
                volatility_pct=round(volatility_pct, 6),
            ))

        return points

    @classmethod
    def build_regime_map(cls, candles: Iterable, **kwargs) -> dict[date, str]:
        return {
            point.timestamp: point.regime
            for point in cls.classify_candles(candles, **kwargs)
        }

    @staticmethod
    def _label_regime(
        trend_pct: float,
        volatility_pct: float,
        bullish_threshold: float,
        bearish_threshold: float,
        sideways_threshold: float,
        volatile_threshold: float,
    ) -> str:
        if volatility_pct >= volatile_threshold and abs(trend_pct) < bullish_threshold:
            return "volatile"
        if trend_pct >= bullish_threshold:
            return "bullish"
        if trend_pct <= bearish_threshold:
            return "bearish"
        if abs(trend_pct) <= sideways_threshold:
            return "sideways"
        return "accumulation" if trend_pct > 0 else "distribution"

    @staticmethod
    def _average_absolute_return(values: list[float]) -> float:
        if len(values) < 2:
            return 0.0
        returns = []
        for previous, current in zip(values, values[1:]):
            if previous > 0:
                returns.append(abs(current / previous - 1))
        return sum(returns) / len(returns) if returns else 0.0

    @staticmethod
    def _to_date(value) -> date:
        if isinstance(value, datetime):
            return value.date()
        if isinstance(value, date):
            return value
        return datetime.fromisoformat(str(value)).date()
