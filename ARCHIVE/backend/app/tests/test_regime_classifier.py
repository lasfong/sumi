from datetime import date, timedelta
from types import SimpleNamespace

from app.domain.regime.regime_classifier import RegimeClassifier


def make_candles(prices):
    base_date = date(2024, 1, 1)
    return [
        SimpleNamespace(timestamp=base_date + timedelta(days=index), close=price)
        for index, price in enumerate(prices)
    ]


def test_regime_classifier_labels_bullish_market():
    candles = make_candles([100 + index for index in range(40)])

    points = RegimeClassifier.classify_candles(candles, lookback=20)

    assert points[-1].regime == "bullish"


def test_regime_classifier_labels_bearish_market():
    candles = make_candles([140 - index for index in range(40)])

    points = RegimeClassifier.classify_candles(candles, lookback=20)

    assert points[-1].regime == "bearish"


def test_regime_classifier_labels_sideways_market():
    candles = make_candles([100, 101, 100.5, 99.8, 100.2] * 8)

    points = RegimeClassifier.classify_candles(candles, lookback=20)

    assert points[-1].regime == "sideways"


def test_regime_classifier_labels_volatile_market():
    candles = make_candles([100, 106, 95, 107, 94, 105, 96, 104, 97, 103] * 4)

    points = RegimeClassifier.classify_candles(candles, lookback=20)

    assert points[-1].regime == "volatile"
