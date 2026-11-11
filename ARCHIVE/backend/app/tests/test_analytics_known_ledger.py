from datetime import date

import pytest

from app.domain.enums import DecisionAction
from app.models.candle import Candle
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.analytics_service import AnalyticsService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService


def _seed_candles(db_session, symbol: str, closes: list[float]) -> None:
    for offset, close in enumerate(closes, start=1):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=date(2024, 1, offset),
            open=close,
            high=close,
            low=close,
            close=close,
            volume=1_000,
        ))


def test_analytics_reconciles_with_known_closed_trade_ledger(db_session):
    _seed_candles(db_session, "LEDGER", [100.0, 90.0, 110.0])
    _seed_candles(db_session, "VNINDEX", [1_000.0, 1_100.0, 900.0])
    db_session.commit()

    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="LEDGER",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 3),
        initial_cash=100_000.0,
    ))
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.BUY, price=100.0, quantity=100.0),
    )
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.CLOSE, price=110.0),
    )

    report = AnalyticsService.get_analytics(db_session, session.id)
    trade = db_session.query(Trade).filter_by(session_id=session.id).one()

    # Buy cash out: 10,000 + 15 fee. Sell cash in: 11,000 - 16.5 fee - 11 tax.
    assert trade.net_pnl == pytest.approx(957.5)
    assert session.current_cash == pytest.approx(100_957.5)
    assert report.total_net_pnl == pytest.approx(957.5)
    assert report.total_trades == 1
    assert report.win_rate == 1.0
    assert report.profit_factor is None
    assert report.expectancy == pytest.approx(957.5)

    assert [point.equity for point in report.equity_curve] == [
        99_985.0,
        98_985.0,
        100_957.5,
    ]
    assert [point.cash for point in report.equity_curve] == [
        89_985.0,
        89_985.0,
        100_957.5,
    ]
    assert [point.holdings_value for point in report.equity_curve] == [
        10_000.0,
        9_000.0,
        0.0,
    ]
    assert report.max_drawdown == pytest.approx(1_000.0)
    assert report.max_drawdown_pct == pytest.approx(1.0)
    assert len(report.drawdown_periods) == 1

    assert [point.value for point in report.benchmark_curve] == [
        100_000.0,
        110_000.0,
        90_000.0,
    ]


def test_no_trade_session_returns_flat_equity_and_benchmark(db_session):
    _seed_candles(db_session, "EMPTY", [50.0, 55.0])
    _seed_candles(db_session, "VNINDEX", [1_000.0, 1_100.0])
    db_session.commit()

    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="EMPTY",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 2),
        initial_cash=25_000.0,
    ))
    ReplayService.next_candle(db_session, session.id)

    report = AnalyticsService.get_analytics(db_session, session.id)

    assert report.total_trades == 0
    assert report.profit_factor is None
    assert [point.equity for point in report.equity_curve] == [25_000.0, 25_000.0]
    assert report.max_drawdown == 0.0
    assert [point.value for point in report.benchmark_curve] == [25_000.0, 27_500.0]
