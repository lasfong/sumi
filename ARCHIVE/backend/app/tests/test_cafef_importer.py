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
    assert candle.close == 96000.0

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
    # Preserves original candle close (96000.0 VND), blocking silent last-wins overwrite
    assert candle.close == 96000.0

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

def test_equity_prices_scaled_1000x_and_volume_preserved(db_session):
    """Equities (FPT, HPG, SSI) must be scaled by 1,000x (kVND -> VND) while volume is preserved."""
    csv_data = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"FPT,20231010,61.9,62.5,61.0,62.0,2513400\n"
        b"HPG,20231010,25.0,26.0,24.5,25.5,15000000\n"
    )
    df = CafeFImporter.parse_file(csv_data, "equities.csv")
    assert len(df) == 2

    # Check parsed DataFrame
    fpt_row = df[df['symbol'] == 'FPT'].iloc[0]
    assert fpt_row['open'] == 61900.0
    assert fpt_row['high'] == 62500.0
    assert fpt_row['low'] == 61000.0
    assert fpt_row['close'] == 62000.0
    assert fpt_row['volume'] == 2513400

    hpg_row = df[df['symbol'] == 'HPG'].iloc[0]
    assert hpg_row['open'] == 25000.0
    assert hpg_row['close'] == 25500.0
    assert hpg_row['volume'] == 15000000

    # Test full import into database
    preview = CafeFImporter.preview(db_session, csv_data, "equities.csv")
    assert preview.can_accept is True
    res = CafeFImporter.import_data(
        db_session, csv_data, "equities.csv",
        confirm_accept=True, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert res.imported_rows == 2

    fpt_candle = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").first()
    assert fpt_candle.close == 62000.0
    assert fpt_candle.volume == 2513400

    fpt_sym = db_session.query(Symbol).filter_by(symbol="FPT").first()
    assert fpt_sym.asset_type == "stock"

def test_index_points_preserved_not_scaled(db_session):
    """Indices (VNINDEX, HNX-INDEX, VN30) must preserve point values without 1000x scaling."""
    csv_data = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"VNINDEX,20231010,1140.5,1150.2,1135.1,1145.3,800000000\n"
        b"HNX-INDEX,20231010,235.1,238.0,234.0,237.5,120000000\n"
        b"VN30,20231010,1155.0,1165.0,1150.0,1160.2,350000000\n"
    )
    df = CafeFImporter.parse_file(csv_data, "indices.csv")
    assert len(df) == 3

    # In parsed DataFrame
    vnindex_row = df[df['symbol'] == 'VNINDEX'].iloc[0]
    assert vnindex_row['open'] == 1140.5
    assert vnindex_row['close'] == 1145.3

    hnx_row = df[df['symbol'] == 'HNX-INDEX'].iloc[0]
    assert hnx_row['open'] == 235.1
    assert hnx_row['close'] == 237.5

    vn30_row = df[df['symbol'] == 'VN30'].iloc[0]
    assert vn30_row['open'] == 1155.0
    assert vn30_row['close'] == 1160.2

    # In database
    preview = CafeFImporter.preview(db_session, csv_data, "indices.csv")
    assert preview.can_accept is True
    res = CafeFImporter.import_data(
        db_session, csv_data, "indices.csv",
        confirm_accept=True, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert res.imported_rows == 3

    candle_vnindex = db_session.query(Candle).filter_by(symbol="VNINDEX", timeframe="1D").first()
    assert candle_vnindex.close == 1145.3

    sym_vnindex = db_session.query(Symbol).filter_by(symbol="VNINDEX").first()
    assert sym_vnindex.asset_type == "index"

def test_reverse_date_order_automatically_sorted(db_session):
    """Files arriving in reverse date order (DF-01) must be sorted chronologically and accepted."""
    # 2023-10-13 (Fri) -> 2023-10-12 (Thu) -> 2023-10-11 (Wed) -> 2023-10-10 (Tue)
    csv_data = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"FPT,20231013,96.0,97.0,95.5,96.5,100000\n"
        b"FPT,20231012,95.0,96.5,94.5,96.0,200000\n"
        b"FPT,20231011,94.0,95.5,93.5,94.5,150000\n"
        b"FPT,20231010,93.0,94.5,92.5,93.5,120000\n"
    )
    # 1. parse_file produces chronologically ascending DataFrame
    df = CafeFImporter.parse_file(csv_data, "reverse_fpt.csv")
    dates = [str(d) for d in df['timestamp'].tolist()]
    assert dates == ['2023-10-10', '2023-10-11', '2023-10-12', '2023-10-13']

    # 2. preview recognizes 0 out-of-order and enables acceptance
    preview = CafeFImporter.preview(db_session, csv_data, "reverse_fpt.csv")
    assert preview.out_of_order_count == 0
    assert preview.parsed_count == 4
    assert preview.can_accept is True

    # 3. accept succeeds
    res = CafeFImporter.import_data(
        db_session, csv_data, "reverse_fpt.csv",
        confirm_accept=True, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert res.imported_rows == 4
    assert res.skipped_rows == 0

    candles = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").order_by(Candle.timestamp.asc()).all()
    assert len(candles) == 4
    assert candles[0].close == 93500.0
    assert candles[3].close == 96500.0

def test_graceful_skip_invalid_and_weekend_anomaly_rows(db_session):
    """Invalid and weekend anomaly rows (DF-02) are skipped with warnings, allowing valid rows to be accepted."""
    # Mon 2023-10-09 (valid), Sat 2023-10-14 (weekend), Tue 2023-10-10 (valid), low > high (invalid), Wed 2023-10-11 (valid)
    csv_data = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"FPT,20231009,92.0,93.0,91.5,92.5,100000\n"
        b"FPT,20231014,95.0,96.0,94.0,95.5,50000\n"
        b"FPT,20231010,93.0,94.0,92.5,93.5,120000\n"
        b"FPT,20231012,94.0,93.0,95.0,94.0,110000\n"
        b"FPT,20231011,93.5,94.5,93.0,94.0,130000\n"
    )
    preview = CafeFImporter.preview(db_session, csv_data, "mixed_anomalies.csv")
    # File has 3 valid weekday rows and 2 rejected rows (1 weekend + 1 invalid low > high)
    assert preview.can_accept is True
    assert preview.parsed_count == 3
    assert preview.rejected_count == 2

    res = CafeFImporter.import_data(
        db_session, csv_data, "mixed_anomalies.csv",
        confirm_accept=True, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert res.imported_rows == 3
    assert res.skipped_rows == 2
    assert len(res.warnings) >= 2
    assert any("cuối tuần" in w.message.lower() for w in res.warnings)

    candles = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").all()
    assert len(candles) == 3

def test_mixed_equities_and_indices_file_scaling(db_session):
    """In a mixed file with equities and indices, equities scale by 1000x while index points are preserved."""
    csv_data = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"VNINDEX,20231010,1140.5,1150.2,1135.1,1145.3,800000000\n"
        b"FPT,20231010,61.9,62.5,61.0,62.0,2513400\n"
    )
    preview = CafeFImporter.preview(db_session, csv_data, "mixed.csv")
    assert preview.can_accept is True
    assert preview.parsed_count == 2

    res = CafeFImporter.import_data(
        db_session, csv_data, "mixed.csv",
        confirm_accept=True, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert res.imported_rows == 2

    vnindex_candle = db_session.query(Candle).filter_by(symbol="VNINDEX", timeframe="1D").first()
    assert vnindex_candle.close == 1145.3

    fpt_candle = db_session.query(Candle).filter_by(symbol="FPT", timeframe="1D").first()
    assert fpt_candle.close == 62000.0
