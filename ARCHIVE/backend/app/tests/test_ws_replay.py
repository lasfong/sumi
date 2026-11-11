from datetime import datetime, timezone
from types import SimpleNamespace

from app.api.ws_replay import ConnectionManager, format_candle


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


def test_stale_websocket_disconnect_preserves_replacement_and_playback():
    manager = ConnectionManager()
    stale_socket = object()
    replacement_socket = object()
    playback_task = SimpleNamespace(cancelled=False)
    playback_task.cancel = lambda: setattr(playback_task, "cancelled", True)
    manager.active_connections[7] = replacement_socket
    manager.playback_tasks[7] = playback_task

    manager.disconnect(7, stale_socket)

    assert manager.active_connections[7] is replacement_socket
    assert manager.playback_tasks[7] is playback_task
    assert playback_task.cancelled is False

    manager.disconnect(7, replacement_socket)

    assert 7 not in manager.active_connections
    assert 7 not in manager.playback_tasks
    assert playback_task.cancelled is True
