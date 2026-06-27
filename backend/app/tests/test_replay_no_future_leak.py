import pytest
from datetime import date
from app.models.candle import Candle
from app.services.replay_service import ReplayService
from app.schemas.replay_schema import ReplaySessionCreate
from app.domain.enums import SessionMode

def test_replay_no_future_leak(db_session):
    # Insert 5 candles
    base_date = date(2023, 10, 1)
    for i in range(5):
        candle = Candle(
            symbol="TEST", timeframe="1D", timestamp=date(2023, 10, i+1),
            open=10+i, high=12+i, low=8+i, close=11+i, volume=1000
        )
        db_session.add(candle)
    db_session.commit()
    
    # Create session
    session_in = ReplaySessionCreate(
        symbol="TEST", start_date=date(2023, 10, 1), end_date=date(2023, 10, 5)
    )
    session = ReplayService.create_session(db_session, session_in)
    
    # Initial state (index 0) -> Should return 1 candle
    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 1
    assert candles[0].timestamp.date() == date(2023, 10, 1)
    
    # Next (index 1) -> Should return 2 candles
    ReplayService.next_candle(db_session, session.id)
    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 2
    assert candles[-1].timestamp.date() == date(2023, 10, 2)
    
    # Next (index 2) -> Should return 3 candles
    ReplayService.next_candle(db_session, session.id)
    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 3
    assert candles[-1].timestamp.date() == date(2023, 10, 3)
    
    # Previous (index 1) -> Should return 2 candles
    ReplayService.previous_candle(db_session, session.id)
    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 2
    assert candles[-1].timestamp.date() == date(2023, 10, 2)
