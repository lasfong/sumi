from datetime import date, timedelta

from app.models.candle import Candle
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
