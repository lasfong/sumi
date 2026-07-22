from datetime import datetime, timezone
from types import SimpleNamespace

from app.api.ws_replay import format_candle


def candle(timestamp: datetime) -> SimpleNamespace:
    return SimpleNamespace(
        timestamp=timestamp,
        open=100.0,
        high=110.0,
        low=90.0,
        close=105.0,
        volume=1_000.0,
    )


def test_format_candle_treats_naive_daily_timestamp_as_utc():
    result = format_candle(candle(datetime(2023, 12, 22)))

    assert result["time"] == 1_703_203_200
    assert datetime.fromtimestamp(result["time"], timezone.utc).date().isoformat() == "2023-12-22"


def test_format_candle_preserves_aware_timestamp_instant():
    result = format_candle(candle(datetime(2023, 12, 22, tzinfo=timezone.utc)))

    assert result["time"] == 1_703_203_200
