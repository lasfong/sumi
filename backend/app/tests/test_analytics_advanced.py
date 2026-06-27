import pytest
from datetime import datetime, timedelta
from unittest.mock import MagicMock
from app.services.analytics_service import AnalyticsService

def test_advanced_analytics_metrics():
    # Mocking db session
    db_mock = MagicMock()
    
    # Create fake trades
    class MockTrade:
        def __init__(self, net_pnl, exit_date):
            self.net_pnl = net_pnl
            self.exit_date = exit_date
            self.quantity = 100
            self.setup_type = "Breakout"
            self.initial_stop_loss = 90
            self.entry_price = 100
            
    # Sequence of returns: +500, -200, +800, -300
    base_date = datetime.now()
    trades = [
        MockTrade(500, base_date - timedelta(days=4)),
        MockTrade(-200, base_date - timedelta(days=3)),
        MockTrade(800, base_date - timedelta(days=2)),
        MockTrade(-300, base_date - timedelta(days=1)),
    ]
    
    # Configure mock
    query_mock = MagicMock()
    query_mock.filter.return_value.order_by.return_value.all.return_value = trades
    db_mock.query.return_value = query_mock
    
    # Run analytics
    response = AnalyticsService.get_analytics(db_mock, session_id=1)
    
    # Verify basic stats
    assert response.total_trades == 4
    assert response.win_rate == 0.5
    assert response.total_net_pnl == 800
    
    # Verify advanced metrics
    assert response.sharpe_ratio is not None
    assert response.sortino_ratio is not None
    assert response.sqn is not None
    
    # Average PnL = 800 / 4 = 200
    # Variance = ( (500-200)^2 + (-200-200)^2 + (800-200)^2 + (-300-200)^2 ) / 4
    # Variance = ( 90000 + 160000 + 360000 + 250000 ) / 4 = 860000 / 4 = 215000
    # StdDev = sqrt(215000) ~= 463.68
    # Sharpe = 200 / 463.68 ~= 0.4313
    assert abs(response.sharpe_ratio - 0.4313) < 0.01
    
    # Equity Curve should have 4 points
    assert response.equity_curve is not None
    assert len(response.equity_curve) == 4
    
    # Equity progression:
    # Initial: 100000
    # T1: 100500, Peak: 100500, DD: 0
    # T2: 100300, Peak: 100500, DD: 200
    # T3: 101100, Peak: 101100, DD: 0
    # T4: 100800, Peak: 101100, DD: 300
    assert response.equity_curve[1].drawdown == 200
    assert response.equity_curve[3].drawdown == 300
    assert response.max_drawdown == 300
