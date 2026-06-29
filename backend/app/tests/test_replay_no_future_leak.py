from datetime import date
from app.models.candle import Candle
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction
from app.models.decision import Decision

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


def test_previous_candle_keeps_existing_decisions_without_future_candles(db_session):
    base_date = date(2024, 3, 1)
    for i in range(5):
        db_session.add(Candle(
            symbol="REWIND",
            timeframe="1D",
            timestamp=base_date.replace(day=base_date.day + i),
            open=20 + i,
            high=22 + i,
            low=19 + i,
            close=21 + i,
            volume=1000,
        ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol="REWIND", start_date=date(2024, 3, 1), end_date=date(2024, 3, 5)),
    )

    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.HOLD, reason="Decision at index 2"),
    )

    ReplayService.previous_candle(db_session, session.id)

    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 2
    assert candles[-1].timestamp.date() == date(2024, 3, 2)

    decisions = db_session.query(Decision).filter_by(session_id=session.id).all()
    assert len(decisions) == 1
    assert decisions[0].candle_index == 2


def test_list_sessions_returns_recent_sessions_first(db_session):
    base_date = date(2024, 4, 1)
    for i in range(3):
        db_session.add(Candle(
            symbol="RESUME",
            timeframe="1D",
            timestamp=base_date.replace(day=base_date.day + i),
            open=30 + i,
            high=32 + i,
            low=29 + i,
            close=31 + i,
            volume=1000,
        ))
    db_session.commit()

    first = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol="RESUME", start_date=date(2024, 4, 1), end_date=date(2024, 4, 3)),
    )
    second = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol="RESUME", start_date=date(2024, 4, 1), end_date=date(2024, 4, 3)),
    )

    sessions = ReplayService.list_sessions(db_session, limit=1)

    assert [session.id for session in sessions] == [second.id]
    assert first.id != second.id
