import pytest
from datetime import datetime, timedelta
from unittest.mock import MagicMock
from app.services.analytics_service import AnalyticsService

def test_advanced_analytics_metrics(db_session):
    from app.models.trade import Trade
    from app.models.replay_session import ReplaySession
    from app.models.candle import Candle
    from datetime import date

    s = ReplaySession(id=1, symbol="TEST", timeframe="1D", adjustment_type="split",
                      start_date=date(2024,1,1), end_date=date(2024,1,10),
                      current_index=0, initial_cash=100000, current_cash=100000,
                      status="active", mode="normal")
    db_session.add(s)

    # Needs some candles to build equity curve
    c1 = Candle(symbol="TEST", timeframe="1D", timestamp=date(2024, 1, 1), open=100, high=100, low=100, close=100, volume=1000)
    c2 = Candle(symbol="TEST", timeframe="1D", timestamp=date(2024, 1, 2), open=100, high=100, low=100, close=100, volume=1000)
    c3 = Candle(symbol="TEST", timeframe="1D", timestamp=date(2024, 1, 3), open=100, high=100, low=100, close=100, volume=1000)
    c4 = Candle(symbol="TEST", timeframe="1D", timestamp=date(2024, 1, 4), open=100, high=100, low=100, close=100, volume=1000)
    db_session.add_all([c1, c2, c3, c4])

    # Create trades
    trades = [
        Trade(id=1, session_id=1, symbol="AAA", entry_date=datetime.now(), exit_date=datetime.now(), net_pnl=500, quantity=100, result="win", entry_price=100, exit_price=105, setup_type="Breakout"),
        Trade(id=2, session_id=1, symbol="AAA", entry_date=datetime.now(), exit_date=datetime.now(), net_pnl=-200, quantity=100, result="loss", entry_price=100, exit_price=98, setup_type="Breakout", mistake_tag="Early Entry"),
        Trade(id=3, session_id=1, symbol="BBB", entry_date=datetime.now(), exit_date=datetime.now(), net_pnl=800, quantity=100, result="win", entry_price=100, exit_price=108, setup_type="Pullback"),
        Trade(id=4, session_id=1, symbol="BBB", entry_date=datetime.now(), exit_date=datetime.now(), net_pnl=-300, quantity=100, result="loss", entry_price=100, exit_price=97, setup_type="Pullback", mistake_tag="FOMO"),
    ]
    db_session.add_all(trades)
    db_session.commit()

    # Run analytics
    response = AnalyticsService.get_analytics(db_session, session_id=1)

    # Verify basic stats
    assert response.total_trades == 4
    assert response.win_rate == 0.5
    assert response.total_net_pnl == 800

    # Verify advanced metrics
    assert response.sharpe_ratio is not None
    assert response.sortino_ratio is not None
    assert response.sqn is not None
    assert isinstance(response.sharpe_ratio, float)

    assert response.equity_curve is not None
    assert response.symbol_performance is not None
    symbol_stats = {row.key: row for row in response.symbol_performance}
    assert symbol_stats["AAA"].trades == 2
    assert symbol_stats["AAA"].net_pnl == 300
    assert symbol_stats["BBB"].average_pnl == 250

    assert response.mistake_performance is not None
    mistake_stats = {row.key: row for row in response.mistake_performance}
    assert mistake_stats["Early Entry"].net_pnl == -200
    assert mistake_stats["FOMO"].win_rate == 0
    assert mistake_stats["No mistake tag"].trades == 2

    assert response.outlier_impact is not None
    assert response.outlier_impact.top_winners_pnl == 1300
    assert response.outlier_impact.top_losers_pnl == -500
    assert response.outlier_impact.median_trade_pnl == pytest.approx(150)
