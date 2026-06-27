import pytest
from datetime import datetime
from app.models.trade import Trade
from app.services.analytics_service import AnalyticsService

def test_analytics_calculations(db_session):
    # Add some mock trades
    t1 = Trade(session_id=1, symbol="TEST", entry_date=datetime.now(), entry_price=100, quantity=100, exit_date=datetime.now(), exit_price=110, net_pnl=1000)
    t2 = Trade(session_id=1, symbol="TEST", entry_date=datetime.now(), entry_price=100, quantity=100, exit_date=datetime.now(), exit_price=95, net_pnl=-500)
    
    db_session.add_all([t1, t2])
    db_session.commit()
    
    analytics = AnalyticsService.get_analytics(db_session, 1)
    
    assert analytics.total_trades == 2
    assert analytics.win_rate == 0.5
    assert analytics.total_net_pnl == 500
    assert analytics.average_win == 1000
    assert analytics.average_loss == 500
    assert analytics.profit_factor == 2.0
