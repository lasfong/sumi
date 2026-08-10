import pytest
import pandas as pd
from datetime import date
from app.services.import_classifier import ImportClassifier, ClassifiedRow

def test_classifier_clean_data():
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
        {"symbol": "SSI", "timestamp": "2026-01-06", "open": 30.5, "high": 32.0, "low": 30.0, "close": 31.5, "volume": 120000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})
    assert can_accept is True
    assert block_reason is None
    assert counts["parsed"] == 2
    assert counts["rejected"] == 0
    assert counts["conflicting"] == 0
    assert counts["duplicate"] == 0

def test_classifier_rejects_weekend():
    # 2026-01-03 is Saturday
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-03", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})
    assert can_accept is False
    assert counts["rejected"] == 1
    assert "cuối tuần" in items[0].reject_reason

def test_classifier_rejects_invalid_ohlc():
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 29.0, "low": 31.0, "close": 30.5, "volume": 100000}, # low > high
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})
    assert can_accept is False
    assert counts["rejected"] == 1
    assert "lớn hơn" in items[0].reject_reason

def test_classifier_detects_conflict_with_db():
    existing_map = {
        ("SSI", "1D", date(2026, 1, 5), "unadjusted"): (30.0, 31.0, 29.5, 30.5, 100000.0)
    }
    # File has same key but different OHLC (high=36.0, close=35.0) vs DB (high=31.0, close=30.5)
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 36.0, "low": 29.5, "close": 35.0, "volume": 100000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, existing_map)
    assert can_accept is False
    assert counts["conflicting"] == 1
    assert "xung đột" in items[0].reject_reason


def test_classifier_detects_duplicate_with_db():
    existing_map = {
        ("SSI", "1D", date(2026, 1, 5), "unadjusted"): (30.0, 31.0, 29.5, 30.5, 100000.0)
    }
    # File has exact same OHLCV
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, existing_map)
    assert counts["duplicate"] == 1
    assert counts["conflicting"] == 0

def test_classifier_detects_out_of_order():
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-06", "open": 30.5, "high": 32.0, "low": 30.0, "close": 31.5, "volume": 120000},
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})
    assert can_accept is False
    assert counts["out_of_order"] == 1

def test_classifier_reports_missing_gap():
    # Jan 5 is Monday, Jan 7 is Wednesday. Jan 6 (Tuesday) is missing gap.
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
        {"symbol": "SSI", "timestamp": "2026-01-07", "open": 31.0, "high": 32.0, "low": 30.5, "close": 31.5, "volume": 110000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})
    assert counts["missing"] == 1
    missing_item = [it for it in items if it.classification == "missing"][0]
    assert missing_item.timestamp == date(2026, 1, 6)



def test_metadata_validation_fail_closed():
    assert ImportClassifier.validate_metadata("unknown-provider", "1D", "unadjusted", "Asia/Ho_Chi_Minh") is not None
    assert ImportClassifier.validate_metadata("cafef", "banana", "unadjusted", "Asia/Ho_Chi_Minh") is not None
    assert ImportClassifier.validate_metadata("cafef", "1D", "mystery", "Asia/Ho_Chi_Minh") is not None
    assert ImportClassifier.validate_metadata("cafef", "1D", "unadjusted", "Mars/Olympus_Mons") is not None

    # Valid metadata returns None
    assert ImportClassifier.validate_metadata("cafef", "1D", "unadjusted", "Asia/Ho_Chi_Minh") is None
    assert ImportClassifier.validate_metadata("manual_upload", "1D", "adjusted", "Asia/Ho_Chi_Minh") is None

