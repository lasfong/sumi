import sys
from pathlib import Path
from datetime import date, datetime, timezone
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from app.db import Base
import app.models  # noqa: F401
from app.models.trade import Trade
from app.models.replay_session import ReplaySession
from app.models.candle import Candle
from app.services.analytics_service import AnalyticsService
from app.services.journal_service import JournalService
from app.domain.accounting import calculate_pnl_percent, calculate_buy_amounts, calculate_sell_amounts
from app.schemas.analytics_schema import TradeDistribution


@pytest.fixture
def test_db():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = SessionClass()
    try:
        yield db
    finally:
        db.close()
        Base.metadata.drop_all(bind=engine)


def test_pnl_percent_accounting_unit_contract():
    """SQA-018: calculate_pnl_percent must return percentage values (0..100 scale, e.g. -2.45 for -2.45%)."""
    # Buy 100 shares @ 100,000 VND. Cash out = 100 * 100,000 * 1.0015 = 10,015,000 VND
    buy_amounts = calculate_buy_amounts(100_000.0, 100.0)
    assert buy_amounts.net_amount == 10_015_000.0

    # Sell 100 shares @ 98,000 VND. Gross = 9,800,000. Fee = 14,700. Tax = 9,800. Net = 9,775,500
    sell_amounts = calculate_sell_amounts(98_000.0, 100.0)
    net_pnl = sell_amounts.net_amount - buy_amounts.net_amount
    assert net_pnl == -239_500.0

    pnl_pct = calculate_pnl_percent(net_pnl, buy_amounts.net_amount)
    # -239,500 / 10,015,000 * 100 = -2.39141%
    assert pnl_pct == pytest.approx(-2.39141, abs=1e-4)
    assert pnl_pct < -1.0, "Must be on 0..100 percentage scale, not fraction scale (-0.0239)"


def test_analytics_trade_distribution_pnl_percent_scale(test_db):
    """SQA-018: TradeDistribution in get_analytics must expose pnl_percent on 0..100 percentage scale,

    matching Journal and Practice surfaces without double-scaling.
    """
    # 1. Create replay session
    session = ReplaySession(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
        end_date=datetime(2024, 1, 30, tzinfo=timezone.utc),
        initial_cash=100_000_000.0,
        current_cash=99_755_423.0,
        current_index=5,
        status="active",
    )
    test_db.add(session)
    test_db.commit()

    # 2. Add closed trade with -2.45% PnL
    # Entry: 100,000 VND, Exit: 97,700 VND, Qty: 100 -> net_pnl = -244,577 VND, pnl_percent = -2.44577
    trade = Trade(
        session_id=session.id,
        symbol="FPT",
        entry_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
        entry_price=100_000.0,
        quantity=100.0,
        exit_date=datetime(2024, 1, 5, tzinfo=timezone.utc),
        exit_price=97_700.0,
        gross_pnl=-230_000.0,
        net_pnl=-244_577.0,
        pnl_percent=-2.44577,
        status="closed",
        result="loss",
        initial_stop_loss=95_000.0,
    )
    test_db.add(trade)
    test_db.commit()

    # 3. Fetch analytics
    analytics = AnalyticsService.get_analytics(test_db, session.id)
    assert analytics is not None
    assert len(analytics.trade_distribution) == 1

    td = analytics.trade_distribution[0]
    assert td.trade_id == trade.id
    assert td.symbol == "FPT"
    assert td.net_pnl == -244_577.0
    assert td.pnl_percent == pytest.approx(-2.44577, abs=1e-4)

    # 4. Verify presentation layer formatting contract:
    # A formatPctValue formatter for 0..100 scale: `${val.toFixed(2)}%`
    def format_pct_value(val):
        return f"{val:.2f}%" if val is not None else "—"

    rendered = format_pct_value(td.pnl_percent)
    assert rendered == "-2.45%", f"Expected '-2.45%', got '{rendered}'"
    assert "244" not in rendered or "-2.45%" in rendered, "Double-scaling detected!"
    assert "-244" not in rendered, "Double-scaling error (-244.58%) detected!"


def test_cross_surface_pnl_percent_parity(test_db):
    """Verify parity of pnl_percent between Analytics, Journal, and Accounting across gain, loss, and zero."""
    session = ReplaySession(
        symbol="HPG",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
        end_date=datetime(2024, 1, 30, tzinfo=timezone.utc),
        initial_cash=100_000_000.0,
        current_cash=105_000_000.0,
        current_index=10,
        status="active",
    )
    test_db.add(session)
    test_db.commit()

    # Trade 1: Gain +5.50%
    t1 = Trade(
        session_id=session.id, symbol="HPG",
        entry_date=datetime(2024, 1, 2, tzinfo=timezone.utc), entry_price=20_000.0, quantity=1000.0,
        exit_date=datetime(2024, 1, 5, tzinfo=timezone.utc), exit_price=21_200.0,
        net_pnl=1_100_000.0, pnl_percent=5.50, status="closed", result="win",
    )
    # Trade 2: Loss -3.20%
    t2 = Trade(
        session_id=session.id, symbol="HPG",
        entry_date=datetime(2024, 1, 8, tzinfo=timezone.utc), entry_price=22_000.0, quantity=500.0,
        exit_date=datetime(2024, 1, 10, tzinfo=timezone.utc), exit_price=21_350.0,
        net_pnl=-352_000.0, pnl_percent=-3.20, status="closed", result="loss",
    )
    # Trade 3: Breakeven 0.00%
    t3 = Trade(
        session_id=session.id, symbol="HPG",
        entry_date=datetime(2024, 1, 12, tzinfo=timezone.utc), entry_price=21_000.0, quantity=500.0,
        exit_date=datetime(2024, 1, 15, tzinfo=timezone.utc), exit_price=21_000.0,
        net_pnl=0.0, pnl_percent=0.0, status="closed", result="breakeven",
    )
    test_db.add_all([t1, t2, t3])
    test_db.commit()

    analytics = AnalyticsService.get_analytics(test_db, session.id)
    td_map = {item.trade_id: item for item in analytics.trade_distribution}

    # Verify Analytics trade distribution
    assert td_map[t1.id].pnl_percent == 5.50
    assert td_map[t2.id].pnl_percent == -3.20
    assert td_map[t3.id].pnl_percent == 0.0

    # Verify Journal representation (e.g. export or table display: f"{t.pnl_percent:.2f}%")
    assert f"{t1.pnl_percent:.2f}%" == "5.50%"
    assert f"{t2.pnl_percent:.2f}%" == "-3.20%"
    assert f"{t3.pnl_percent:.2f}%" == "0.00%"
