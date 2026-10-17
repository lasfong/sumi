from datetime import date, datetime, timedelta, timezone
import io

from app.domain.market.calendar import TradingCalendar
from app.models.candle import Candle
from app.models.symbol import Symbol
from scripts.seed_demo import DEMO_SYMBOLS, seed_symbol


def _seed_symbol_with_daily_candles(db_session, ticker: str = "FPT", count: int = 80):
    sym = Symbol(
        symbol=ticker,
        exchange="HOSE",
        company_name=f"{ticker} Corp",
        sector="Technology",
        industry="Software",
        asset_type="stock",
        is_active=True,
    )
    db_session.merge(sym)
    db_session.flush()

    cal = TradingCalendar()
    cur = date(2024, 1, 2)
    added = 0
    while added < count:
        if cal.is_trading_day(cur):
            base = 100000.0 + added * 200.0
            db_session.add(
                Candle(
                    symbol=ticker,
                    timeframe="1D",
                    timestamp=datetime(cur.year, cur.month, cur.day, tzinfo=timezone.utc),
                    open=base,
                    high=base + 1500.0,
                    low=base - 1000.0,
                    close=base + 500.0,
                    volume=1_200_000 + added * 5000,
                    source="cafef",
                    adjustment_type="unadjusted",
                )
            )
            added += 1
        cur += timedelta(days=1)
    db_session.commit()
    return sym


def test_def004_invalid_inputs_return_structured_4xx_instead_of_500(client, db_session):
    """DEF-004: Invalid SL/TP geometry on /plan-sizing and incomplete strategy dict on /backtest/run return 400/422."""
    _seed_symbol_with_daily_candles(db_session, "FPT", count=40)

    create_resp = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "FPT",
            "timeframe": "1D",
            "start_date": "2024-01-02",
            "end_date": "2024-03-29",
            "initial_capital": 100_000_000,
        },
    )
    assert create_resp.status_code == 200
    session_id = create_resp.json()["id"]

    # Invalid LONG geometry: stop_loss >= entry_price
    bad_long = client.post(
        f"/api/replay/sessions/{session_id}/plan-sizing",
        json={
            "direction": "LONG",
            "entry_price": 100_000,
            "stop_loss": 105_000,
            "target_price": 115_000,
            "risk_percent": 1.5,
        },
    )
    assert bad_long.status_code == 400
    body_long = bad_long.json()
    assert body_long["code"] == "VALIDATION_ERROR"
    assert "stop loss must be strictly below entry price" in body_long["message"].lower()

    # Invalid SHORT geometry: stop_loss <= entry_price
    bad_short = client.post(
        f"/api/replay/sessions/{session_id}/plan-sizing",
        json={
            "direction": "SHORT",
            "entry_price": 100_000,
            "stop_loss": 95_000,
            "target_price": 85_000,
            "risk_percent": 1.5,
        },
    )
    assert bad_short.status_code == 400
    body_short = bad_short.json()
    assert body_short["code"] == "VALIDATION_ERROR"
    assert "stop loss must be strictly above entry price" in body_short["message"].lower()

    # Incomplete strategy dict on /api/backtest/run
    bad_bt = client.post(
        "/api/backtest/run",
        json={
            "symbol": "FPT",
            "start_date": "2024-01-02",
            "end_date": "2024-03-29",
            "strategy": {"name": "bad"},
        },
    )
    assert bad_bt.status_code == 422
    body_bt = bad_bt.json()
    assert body_bt["code"] == "VALIDATION_ERROR"
    assert "entry_rules" in body_bt["message"] or any(
        "entry_rules" in err.get("field", "") for err in body_bt.get("field_errors", [])
    )


def test_def005_seed_demo_respects_trading_calendar_holidays_and_weekends(db_session):
    """DEF-005: Demo seeding uses TradingCalendar and never creates candles on holidays or weekends."""
    cal = TradingCalendar()
    count = seed_symbol(DEMO_SYMBOLS[0], sessions=520, db=db_session)
    assert count == 520

    candles = db_session.query(Candle).order_by(Candle.timestamp.asc()).all()
    assert len(candles) == 520

    candle_dates = {c.timestamp.date() for c in candles}

    # Verify zero weekend candles
    assert all(d.weekday() < 5 for d in candle_dates)

    # Verify every seeded date is an authoritative trading day
    assert all(cal.is_trading_day(d) for d in candle_dates)

    # Explicitly check official 2024 holidays (Tet, Reunification/Labor Day, National Day)
    official_2024_holidays = [
        date(2024, 1, 1),
        date(2024, 2, 8),
        date(2024, 2, 9),
        date(2024, 2, 12),
        date(2024, 2, 13),
        date(2024, 2, 14),
        date(2024, 4, 18),
        date(2024, 4, 29),
        date(2024, 4, 30),
        date(2024, 5, 1),
        date(2024, 9, 2),
        date(2024, 9, 3),
    ]
    for holiday in official_2024_holidays:
        assert holiday not in candle_dates, f"Found seeded candle on official holiday {holiday}"


def test_def006_replay_session_after_import_rollback_reports_zero_candles_and_preserves_session(client):
    """DEF-006: Rolling back imported symbol candles marks session as has_candles=False without deleting session records."""
    csv_lines = ["<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>"]
    cal = TradingCalendar()
    cur = date(2024, 1, 2)
    added = 0
    while added < 15:
        if cal.is_trading_day(cur):
            ymd = cur.strftime("%Y%m%d")
            csv_lines.append(f"QAROLL,{ymd},25.0,26.0,24.5,25.5,500000")
            added += 1
        cur += timedelta(days=1)
    csv_bytes = "\n".join(csv_lines).encode("utf-8")

    preview_resp = client.post(
        "/api/import/preview",
        files={"file": ("qaroll.csv", io.BytesIO(csv_bytes), "text/csv")},
        data={
            "source_type": "cafef",
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "confirm": "true",
        },
    )
    assert preview_resp.status_code == 200
    preview_data = preview_resp.json()
    run_id = preview_data["run_id"]
    sha256 = preview_data["content_sha256"]

    accept_resp = client.post(
        f"/api/import/runs/{run_id}/accept",
        json={"content_sha256": sha256},
    )
    assert accept_resp.status_code == 200

    # Create replay session on QAROLL
    sess_resp = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "QAROLL",
            "timeframe": "1D",
            "start_date": "2024-01-02",
            "end_date": "2024-02-05",
            "initial_capital": 100_000_000,
        },
    )
    assert sess_resp.status_code == 200
    sess_data = sess_resp.json()
    session_id = sess_data["id"]
    assert sess_data["has_candles"] is True
    assert sess_data["available_candles"] == 15

    # Roll back the import run
    rb_resp = client.post(f"/api/import/runs/{run_id}/rollback")
    assert rb_resp.status_code == 200

    # Session record is preserved, but exposes has_candles=False & available_candles=0 so client can recover cleanly
    after_resp = client.get(f"/api/replay/sessions/{session_id}")
    assert after_resp.status_code == 200
    after_data = after_resp.json()
    assert after_data["id"] == session_id
    assert after_data["symbol"] == "QAROLL"
    assert after_data["available_candles"] == 0
    assert after_data["has_candles"] is False


def test_def008_open_position_scaling_uses_total_account_equity_for_risk_budget(client, db_session):
    """DEF-008: When scaling with an open position, /plan-sizing computes max_risk_amount from total equity (cash + open_position_value)."""
    _seed_symbol_with_daily_candles(db_session, "FPT", count=30)

    create_resp = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "FPT",
            "timeframe": "1D",
            "start_date": "2024-01-02",
            "end_date": "2024-03-15",
            "initial_capital": 100_000_000,
        },
    )
    assert create_resp.status_code == 200
    session_id = create_resp.json()["id"]

    # Get initial practice snapshot and submit a BUY order of 500 shares (~50M VND)
    state0 = client.get(f"/api/replay/sessions/{session_id}/practice-state").json()
    price0 = state0["current_price"]

    buy_resp = client.post(
        f"/api/replay/sessions/{session_id}/decisions",
        json={
            "action": "BUY",
            "quantity": 500,
            "price": price0,
            "order_type": "MARKET_NEXT_OPEN",
        },
    )
    assert buy_resp.status_code == 200

    # Advance 1 bar so the order fills at next open and creates an open LONG position
    step_resp = client.post(f"/api/replay/sessions/{session_id}/next?steps=1")
    assert step_resp.status_code == 200

    state1 = client.get(f"/api/replay/sessions/{session_id}/practice-state").json()
    current_cash = float(state1["current_cash"])
    open_pos_val = sum(float(p["current_price"]) * float(p["quantity"]) for p in state1["positions"])
    assert open_pos_val > 0
    expected_equity = round(current_cash + open_pos_val, 2)

    entry_price = float(state1["current_price"])
    stop_loss = round(entry_price * 0.95, 2)
    target_price = round(entry_price * 1.10, 2)
    risk_pct = 2.0

    plan_resp = client.post(
        f"/api/replay/sessions/{session_id}/plan-sizing",
        json={
            "direction": "LONG",
            "entry_price": entry_price,
            "stop_loss": stop_loss,
            "target_price": target_price,
            "risk_percent": risk_pct,
        },
    )
    assert plan_resp.status_code == 200
    plan = plan_resp.json()

    assert abs(plan["available_cash"] - current_cash) < 0.05
    assert abs(plan["account_equity"] - expected_equity) < 0.05

    expected_max_risk_from_equity = round(expected_equity * (risk_pct / 100.0), 2)
    cash_only_risk = round(current_cash * (risk_pct / 100.0), 2)
    assert abs(plan["max_risk_amount"] - expected_max_risk_from_equity) < 0.05
    assert plan["max_risk_amount"] > cash_only_risk


def test_def012_money_flow_bb_signals_accept_registry_default_parameters(client, db_session):
    """DEF-012: bb.direction_rising and bb.regime_positive calculate cleanly using registry default_parameters."""
    _seed_symbol_with_daily_candles(db_session, "FPT", count=65)

    create_resp = client.post(
        "/api/replay/sessions",
        json={
            "symbol": "FPT",
            "timeframe": "1D",
            "start_date": "2024-01-02",
            "end_date": "2024-04-25",
            "initial_capital": 100_000_000,
        },
    )
    assert create_resp.status_code == 200
    session_id = create_resp.json()["id"]

    # Step forward 45 bars so BB money-flow indicators have sufficient lookback
    client.post(f"/api/replay/sessions/{session_id}/next?steps=45")

    reg_resp = client.get("/api/signals/registry")
    assert reg_resp.status_code == 200
    signals_by_name = {s["name"]: s for s in reg_resp.json()["signals"]}

    dir_defaults = signals_by_name["bb.direction_rising"]["default_parameters"]
    reg_defaults = signals_by_name["bb.regime_positive"]["default_parameters"]
    assert dir_defaults == {"horizon": "T20", "min_quality": "HIGH"}
    assert reg_defaults == {"horizon": "T20", "min_quality": "HIGH"}

    calc_resp = client.post(
        f"/api/signals/replay/{session_id}/calculate",
        json={
            "signals": [
                {"name": "bb.direction_rising", "params": dir_defaults},
                {"name": "bb.regime_positive", "params": reg_defaults},
            ],
        },
    )
    assert calc_resp.status_code == 200
    results = calc_resp.json()["results"]
    assert len(results) == 2
    assert {r["signal_name"] for r in results} == {"bb.direction_rising", "bb.regime_positive"}
    for res in results:
        assert len(res["points"]) > 0
