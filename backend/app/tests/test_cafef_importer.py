import pytest
import pandas as pd
from app.services.cafef_importer import CafeFImporter
from app.models.candle import Candle
from app.models.symbol import Symbol

def test_parse_valid_csv():
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNINDEX,20231010,1140.5,1150.2,1135.1,1145.3,800000000\n"
    df = CafeFImporter.parse_file(csv_data, "test.csv")

    assert len(df) == 1
    assert df.iloc[0]['symbol'] == 'VNINDEX'
    assert str(df.iloc[0]['timestamp']) == '2023-10-10'
    assert df.iloc[0]['open'] == 1140.5

def test_import_data_without_confirmation_raises_error_and_does_not_mutate_data(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,2500000\n"

    # Invocation without confirm_accept raises RuntimeError and does not mutate candles/symbols
    with pytest.raises(RuntimeError) as exc_info:
        CafeFImporter.import_data(db_session, csv_data, "fpt.csv")

    assert "Tự động chấp nhận bị cấm" in str(exc_info.value)
    assert db_session.query(Candle).count() == 0
    assert db_session.query(Symbol).count() == 0

def test_import_data_with_explicit_confirmation_success(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,2500000\n"

    preview = CafeFImporter.preview(db_session, csv_data, "fpt.csv")
    assert preview.can_accept is True

    response = CafeFImporter.import_data(
        db_session,
        csv_data,
        "fpt.csv",
        confirm_accept=True,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    assert response.imported_rows == 1
    assert response.skipped_rows == 0
    assert response.symbols_count == 1

    # Check DB
    symbol = db_session.query(Symbol).first()
    assert symbol.symbol == "FPT"

    candle = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").first()
    assert candle.symbol == "FPT"
    assert candle.close == 96.0

def test_data_quality_negative_volume(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,-100\n"
    preview = CafeFImporter.preview(db_session, csv_data, "bad.csv")
    assert preview.can_accept is False

    response = CafeFImporter.import_data(
        db_session,
        csv_data,
        "bad.csv",
        confirm_accept=True,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    assert response.imported_rows == 0
    assert response.skipped_rows == 1
    assert len(response.warnings) >= 1

def test_import_is_idempotent_and_quarantines_conflicts(db_session):
    original = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,2500000\n"
    corrected = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,98.5,94.5,98.0,3000000\n"

    p1 = CafeFImporter.preview(db_session, original, "fpt.csv")
    first = CafeFImporter.import_data(db_session, original, "fpt.csv", confirm_accept=True, run_id=p1.run_id, content_sha256=p1.content_sha256)

    p2 = CafeFImporter.preview(db_session, original, "fpt.csv")
    second = CafeFImporter.import_data(db_session, original, "fpt.csv", confirm_accept=True, run_id=p2.run_id, content_sha256=p2.content_sha256)

    p3 = CafeFImporter.preview(db_session, corrected, "fpt.csv")
    assert p3.can_accept is False
    third = CafeFImporter.import_data(db_session, corrected, "fpt.csv", confirm_accept=True, run_id=p3.run_id, content_sha256=p3.content_sha256)

    assert first.imported_rows == 1
    assert second.imported_rows == 0  # Idempotent noop
    assert third.imported_rows == 0   # Conflicting data blocked/quarantined under PRO-DATA-03
    assert db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").count() == 1
    candle = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").one()
    # Preserves original candle close (96.0), blocking silent last-wins overwrite
    assert candle.close == 96.0

def test_import_skips_unparseable_timestamp(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,not-a-date,95.0,96.5,94.5,96.0,2500000\n"

    preview = CafeFImporter.preview(db_session, csv_data, "bad-date.csv")
    assert preview.can_accept is False

    response = CafeFImporter.import_data(
        db_session,
        csv_data,
        "bad-date.csv",
        confirm_accept=True,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    assert response.imported_rows == 0
    assert response.skipped_rows == 1
    assert any("ngày" in warning.message.lower() or "tập tin" in warning.message.lower() for warning in response.warnings)
