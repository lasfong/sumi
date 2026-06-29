from datetime import date, timedelta
import json

from app.models.candle import Candle
from app.models.replay_session import ReplaySession
from app.services.scanner_history_service import ScannerHistoryService
from app.services.scanner_service import ScannerService


def seed_trending_candles(db_session, symbol: str, base_date: date, count: int = 40):
    for index in range(count):
        price = 100 + index
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=base_date + timedelta(days=index),
            open=price,
            high=price + 1,
            low=price - 1,
            close=price,
            volume=1000000,
        ))


def test_scanner_finds_entry_signals_with_regime(db_session):
    base_date = date(2024, 1, 1)
    seed_trending_candles(db_session, "SCAN_AAA", base_date)
    seed_trending_candles(db_session, "VNINDEX", base_date)
    db_session.commit()

    config = {
        "symbols": ["SCAN_AAA"],
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=40)),
        "strategy": {
            "name": "Scanner SMA Trend",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["sma_short", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_short", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
    }

    result = ScannerService().run_scan(db_session, config)

    assert result["status"] == "succeeded"
    assert result["total_results"] > 0
    assert result["results"][0]["symbol"] == "SCAN_AAA"
    assert result["results"][0]["signal_type"] == "entry"
    assert result["results"][0]["regime"] != "Unknown"


def test_scanner_rejects_unknown_rule_identifier(db_session):
    base_date = date(2024, 1, 1)
    seed_trending_candles(db_session, "SCAN_BAD", base_date)
    db_session.commit()

    config = {
        "symbols": ["SCAN_BAD"],
        "start_date": str(base_date),
        "end_date": str(base_date + timedelta(days=40)),
        "strategy": {
            "name": "Bad Scanner Rule",
            "indicators": [{"name": "sma_short", "type": "sma", "length": 5}],
            "entry_rules": [{"dsl": {"gt": ["missing_indicator", 0]}}],
            "exit_rules": [{"dsl": {"lt": ["sma_short", 0]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
    }

    result = ScannerService().run_scan(db_session, config)

    assert result["status"] == "failed"
    assert result["error_code"] == "INVALID_RULE"
    assert "missing_indicator" in result["message"]


def test_scanner_signal_can_create_replay_session(db_session):
    base_date = date(2024, 1, 1)
    seed_trending_candles(db_session, "SCAN_REPLAY", base_date, count=60)
    db_session.commit()

    result = ScannerService().create_replay_session_from_signal(db_session, {
        "symbol": "SCAN_REPLAY",
        "signal_timestamp": str(base_date + timedelta(days=30)),
        "lookback_days": 10,
        "forward_days": 10,
        "initial_cash": 50_000_000,
    })

    session = result["session"]
    assert session.id is not None
    assert session.symbol == "SCAN_REPLAY"
    assert session.initial_cash == 50_000_000
    assert session.start_date == base_date + timedelta(days=20)
    assert session.end_date == base_date + timedelta(days=40)
    assert session.source_type == "scanner_signal"
    source_payload = json.loads(session.source_payload)
    assert source_payload["symbol"] == "SCAN_REPLAY"
    assert source_payload["signal_timestamp"] == "2024-01-31T00:00:00"
    assert source_payload["lookback_days"] == 10
    assert source_payload["forward_days"] == 10
    assert db_session.query(ReplaySession).filter_by(id=session.id).first() is not None


def test_scanner_history_persists_and_lists_runs(db_session):
    request_config = {
        "symbols": ["FPT", "SSI"],
        "start_date": "2024-01-01",
        "end_date": "2024-02-01",
        "strategy": {"name": "Scanner Strategy"},
    }
    result_payload = {
        "status": "succeeded",
        "total_results": 1,
        "results": [{"symbol": "FPT", "timestamp": "2024-01-10T00:00:00"}],
        "warnings": [],
    }

    saved = ScannerHistoryService.create_run(db_session, request_config, result_payload)
    runs = ScannerHistoryService.list_runs(db_session)

    assert saved["id"] == runs[0]["id"]
    assert runs[0]["label"] == "Scanner Strategy on 2 symbol(s)"
    assert runs[0]["request_config"]["symbols"] == ["FPT", "SSI"]
    assert runs[0]["result_payload"]["total_results"] == 1
