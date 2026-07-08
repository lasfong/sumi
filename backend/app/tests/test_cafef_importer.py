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

def test_import_data_success(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,2500000\n"
    response = CafeFImporter.import_data(db_session, csv_data, "fpt.csv")
    
    assert response.imported_rows == 1
    assert response.skipped_rows == 0
    assert response.symbols_count == 1
    
    # Check DB
    symbol = db_session.query(Symbol).first()
    assert symbol.symbol == "FPT"
    
    candle = db_session.query(Candle).first()
    assert candle.symbol == "FPT"
    assert candle.close == 96.0

def test_data_quality_negative_volume(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,-100\n"
    response = CafeFImporter.import_data(db_session, csv_data, "bad.csv")
    
    # The negative volume row should be skipped
    assert response.imported_rows == 0
    assert response.skipped_rows == 1
    assert len(response.warnings) == 1
    assert "Negative volume" in response.warnings[0].message


def test_import_is_idempotent_and_updates_existing_candle(db_session):
    original = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,96.5,94.5,96.0,2500000\n"
    corrected = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20231010,95.0,98.5,94.5,98.0,3000000\n"

    first = CafeFImporter.import_data(db_session, original, "fpt.csv")
    second = CafeFImporter.import_data(db_session, original, "fpt.csv")
    third = CafeFImporter.import_data(db_session, corrected, "fpt.csv")

    assert first.imported_rows == second.imported_rows == third.imported_rows == 1
    assert db_session.query(Candle).filter_by(symbol="FPT").count() == 1
    candle = db_session.query(Candle).filter_by(symbol="FPT").one()
    assert candle.close == 98.0
    assert candle.volume == 3_000_000


def test_import_skips_unparseable_timestamp(db_session):
    csv_data = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,not-a-date,95.0,96.5,94.5,96.0,2500000\n"

    response = CafeFImporter.import_data(db_session, csv_data, "bad-date.csv")

    assert response.imported_rows == 0
    assert response.skipped_rows == 1
    assert any("timestamp" in warning.message.lower() for warning in response.warnings)
