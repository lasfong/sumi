import pytest
from datetime import datetime, date
from app.models.candle import Candle
from app.models.import_run import WeeklyCandleProvenance
from app.services.weekly_aggregator import WeeklyAggregator

def test_weekly_aggregator_vn_trading_week_v1(db_session):
    # Jan 5, 2026 (Mon) to Jan 9, 2026 (Fri) - 5 days in one week
    c1 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2026, 1, 5), open=100.0, high=105.0, low=99.0, close=102.0, volume=1000, adjustment_type="unadjusted")
    c2 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2026, 1, 6), open=102.0, high=108.0, low=101.0, close=107.0, volume=1500, adjustment_type="unadjusted")
    c3 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2026, 1, 7), open=107.0, high=107.5, low=104.0, close=105.0, volume=1200, adjustment_type="unadjusted")
    c4 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2026, 1, 8), open=105.0, high=106.0, low=103.0, close=104.0, volume=1100, adjustment_type="unadjusted")
    c5 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2026, 1, 9), open=104.0, high=110.0, low=103.5, close=109.0, volume=2000, adjustment_type="unadjusted")
    
    db_session.add_all([c1, c2, c3, c4, c5])
    db_session.commit()

    count = WeeklyAggregator.derive_weekly_candles(db_session, {"VCB"}, {"unadjusted"})
    assert count == 1

    weekly_candle = db_session.query(Candle).filter(Candle.symbol == "VCB", Candle.timeframe == "1W").first()
    assert weekly_candle is not None
    assert weekly_candle.timestamp.date() == date(2026, 1, 9) # timestamp at final included trading date
    assert weekly_candle.open == 100.0 # first open
    assert weekly_candle.high == 110.0 # max high
    assert weekly_candle.low == 99.0   # min low
    assert weekly_candle.close == 109.0 # last close
    assert weekly_candle.volume == 6800.0 # sum volume

    prov = db_session.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "VCB").first()
    assert prov is not None
    assert prov.week_start_date == "2026-01-05"
    assert prov.rule_version == "VN_TRADING_WEEK_V1"
