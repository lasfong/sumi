import math
from datetime import date, timedelta

import pandas as pd
import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from app.db import Base
from app.dependencies import get_db
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.main import app
from app.models.candle import Candle


def _seed_deterministic_candles(db, symbol: str, base_date: date, count: int = 90) -> pd.DataFrame:
    rows = []
    for index in range(count):
        close = 100 + index * 0.35 + math.sin(index / 4) * 3.5
        row = {
            "timestamp": base_date + timedelta(days=index),
            "open": close - 0.7,
            "high": close + 1.1,
            "low": close - 1.3,
            "close": close,
            "volume": 1_000_000 + index * 100,
        }
        rows.append(row)
        db.add(Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            **row,
        ))
    db.commit()
    return pd.DataFrame(rows)


def _make_client(db):
    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    return TestClient(app)


def _value_by_timestamp(api_rows: list[dict], column_prefix: str) -> dict[str, float]:
    values = {}
    for row in api_rows:
        column = next(key for key in row if key != "timestamp" and key.lower().startswith(column_prefix))
        if row[column] is not None:
            values[row["timestamp"][:10]] = row[column]
    return values


def test_replay_indicator_api_matches_strategy_indicator_adapter_after_warmup():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()
    symbol = "PARITY"
    base_date = date(2024, 1, 1)
    df = _seed_deterministic_candles(db, symbol, base_date)

    client = _make_client(db)
    try:
        session_response = client.post("/api/replay/sessions", json={
            "symbol": symbol,
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": str(base_date),
            "end_date": str(base_date + timedelta(days=len(df))),
            "initial_cash": 100_000_000,
            "mode": "normal",
        })
        assert session_response.status_code == 200, session_response.text
        session_id = session_response.json()["id"]
        advance_response = client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": len(df) - 1})
        assert advance_response.status_code == 200, advance_response.text

        strategy = load_strategy_from_dict({
            "name": "Indicator Parity",
            "indicators": [
                {"name": "rsi", "type": "rsi", "length": 14},
                {"name": "macd", "type": "macd", "fast": 12, "slow": 26, "signal": 9},
                {"name": "cci", "type": "cci", "length": 20},
            ],
            "entry_rules": [{"dsl": {"gt": ["rsi", 50]}}],
            "exit_rules": [{"dsl": {"lt": ["rsi", 50]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        adapter_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
        dates = [value.isoformat() for value in df["timestamp"]]

        checks = [
            ("rsi", {"indicator": "rsi", "length": 14}, "rsi", "rsi"),
            ("macd_line", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macd_", "macd"),
            ("macd_signal", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macds", "macd"),
            ("macd_hist", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macdh", "macd"),
            ("cci", {"indicator": "cci", "length": 20}, "cci", "cci"),
        ]

        cached_api_payloads = {}
        for adapter_key, params, api_prefix, cache_key in checks:
            if cache_key not in cached_api_payloads:
                response = client.get(f"/api/replay/sessions/{session_id}/indicators", params=params)
                assert response.status_code == 200, response.text
                cached_api_payloads[cache_key] = response.json()["data"]
            api_values = _value_by_timestamp(cached_api_payloads[cache_key], api_prefix)

            compared = 0
            for index, expected in enumerate(adapter_values[adapter_key]):
                if pd.isna(expected):
                    continue
                timestamp = dates[index]
                assert timestamp in api_values
                assert api_values[timestamp] == pytest.approx(float(expected), rel=1e-9, abs=1e-9)
                compared += 1
            assert compared > 0
    finally:
        client.close()
        app.dependency_overrides.clear()
        db.close()
        Base.metadata.drop_all(bind=engine)
