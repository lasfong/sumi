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
                {"name": "sma", "type": "sma", "length": 20},
                {"name": "bbands", "type": "bbands", "length": 20, "std": 2.0},
                {"name": "atr", "type": "atr", "length": 14},
                {"name": "volume_sma", "type": "volume_sma", "length": 20},
            ],
            "entry_rules": [{"dsl": {"gt": ["rsi", 50]}}],
            "exit_rules": [{"dsl": {"lt": ["rsi", 50]}}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        adapter_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
        dates = [value.isoformat() for value in df["timestamp"]]

        bbu_key = next(k for k in adapter_values if k.startswith("bbands_bbu"))
        bbm_key = next(k for k in adapter_values if k.startswith("bbands_bbm"))
        bbl_key = next(k for k in adapter_values if k.startswith("bbands_bbl"))

        checks = [
            ("rsi", {"indicator": "rsi", "length": 14}, "rsi", "rsi"),
            ("macd_line", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macd_", "macd"),
            ("macd_signal", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macds", "macd"),
            ("macd_hist", {"indicator": "macd", "fast": 12, "slow": 26, "signal": 9}, "macdh", "macd"),
            ("cci", {"indicator": "cci", "length": 20}, "cci", "cci"),
            ("sma", {"indicator": "sma", "length": 20}, "sma_", "sma"),
            (bbu_key, {"indicator": "bbands", "length": 20, "std": 2.0}, "bbu_", "bbands"),
            (bbm_key, {"indicator": "bbands", "length": 20, "std": 2.0}, "bbm_", "bbands"),
            (bbl_key, {"indicator": "bbands", "length": 20, "std": 2.0}, "bbl_", "bbands"),
            ("atr", {"indicator": "atr", "length": 14}, "atrr_", "atr"),
            ("volume_sma", {"indicator": "volume_sma", "length": 20}, "volume_sma_", "volume_sma"),
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


def test_replay_indicator_api_non_default_bbands_parity():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()
    symbol = "PARITY_BB"
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
            "name": "Bollinger Non-Default Parity",
            "indicators": [
                {"name": "bbands_225", "type": "bbands", "length": 20, "std": 2.25},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        adapter_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
        dates = [value.isoformat() for value in df["timestamp"]]

        assert "bbands_225_bbu_20_2.25_2.25" in adapter_values
        assert "bbands_225_bbm_20_2.25_2.25" in adapter_values
        assert "bbands_225_bbl_20_2.25_2.25" in adapter_values

        response = client.get(f"/api/replay/sessions/{session_id}/indicators", params={"indicator": "bbands", "length": 20, "std": 2.25})
        assert response.status_code == 200, response.text
        api_data = response.json()["data"]

        api_upper = _value_by_timestamp(api_data, "bbu_")
        api_middle = _value_by_timestamp(api_data, "bbm_")
        api_lower = _value_by_timestamp(api_data, "bbl_")

        for idx, ts in enumerate(dates):
            exp_u = adapter_values["bbands_225_bbu_20_2.25_2.25"][idx]
            exp_m = adapter_values["bbands_225_bbm_20_2.25_2.25"][idx]
            exp_l = adapter_values["bbands_225_bbl_20_2.25_2.25"][idx]
            if pd.isna(exp_u):
                continue
            assert api_upper[ts] == pytest.approx(float(exp_u), rel=1e-9, abs=1e-9)
            assert api_middle[ts] == pytest.approx(float(exp_m), rel=1e-9, abs=1e-9)
            assert api_lower[ts] == pytest.approx(float(exp_l), rel=1e-9, abs=1e-9)
    finally:
        client.close()
        app.dependency_overrides.clear()
        db.close()
        Base.metadata.drop_all(bind=engine)


def test_replay_indicator_api_pro05_momentum_parity():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()
    symbol = "PARITY_MOM"
    base_date = date(2024, 1, 1)
    df = _seed_deterministic_candles(db, symbol, base_date, count=90)
    # Seed VNINDEX benchmark candles
    _seed_deterministic_candles(db, "VNINDEX", base_date, count=90)

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
            "name": "PRO-05 Momentum Parity",
            "indicators": [
                {"name": "mfi", "type": "mfi", "length": 14},
                {"name": "stoch", "type": "stoch", "k": 14, "d": 3, "smooth_k": 3},
                {"name": "adx", "type": "adx", "length": 14},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        adapter_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
        dates = [value.isoformat() for value in df["timestamp"]]

        # 1. MFI
        res_mfi = client.get(f"/api/replay/sessions/{session_id}/indicators", params={"indicator": "mfi", "length": 14})
        assert res_mfi.status_code == 200, res_mfi.text
        api_mfi = _value_by_timestamp(res_mfi.json()["data"], "mfi_")
        for idx, expected in enumerate(adapter_values["mfi"]):
            if pd.isna(expected):
                continue
            ts = dates[idx]
            assert api_mfi[ts] == pytest.approx(float(expected), rel=1e-9, abs=1e-9)

        # 2. Stochastic
        res_stoch = client.get(f"/api/replay/sessions/{session_id}/indicators", params={"indicator": "stoch", "k": 14, "d": 3, "smooth_k": 3})
        assert res_stoch.status_code == 200, res_stoch.text
        api_stoch_k = _value_by_timestamp(res_stoch.json()["data"], "stochk_")
        api_stoch_d = _value_by_timestamp(res_stoch.json()["data"], "stochd_")
        stoch_k_key = next(k for k in adapter_values if k.startswith("stoch_stochk_"))
        stoch_d_key = next(k for k in adapter_values if k.startswith("stoch_stochd_"))
        for idx, ts in enumerate(dates):
            exp_k = adapter_values[stoch_k_key][idx]
            exp_d = adapter_values[stoch_d_key][idx]
            if pd.isna(exp_k) or pd.isna(exp_d):
                continue
            assert api_stoch_k[ts] == pytest.approx(float(exp_k), rel=1e-9, abs=1e-9)
            assert api_stoch_d[ts] == pytest.approx(float(exp_d), rel=1e-9, abs=1e-9)

        # 3. ADX
        res_adx = client.get(f"/api/replay/sessions/{session_id}/indicators", params={"indicator": "adx", "length": 14})
        assert res_adx.status_code == 200, res_adx.text
        api_adx = _value_by_timestamp(res_adx.json()["data"], "adx_")
        api_dmp = _value_by_timestamp(res_adx.json()["data"], "dmp_")
        api_dmn = _value_by_timestamp(res_adx.json()["data"], "dmn_")
        adx_key = next(k for k in adapter_values if k.startswith("adx_adx_"))
        dmp_key = next(k for k in adapter_values if k.startswith("adx_dmp_"))
        dmn_key = next(k for k in adapter_values if k.startswith("adx_dmn_"))
        for idx, ts in enumerate(dates):
            exp_adx = adapter_values[adx_key][idx]
            exp_dmp = adapter_values[dmp_key][idx]
            exp_dmn = adapter_values[dmn_key][idx]
            if pd.isna(exp_adx) or pd.isna(exp_dmp) or pd.isna(exp_dmn):
                continue
            assert api_adx[ts] == pytest.approx(float(exp_adx), rel=1e-9, abs=1e-9)
            assert api_dmp[ts] == pytest.approx(float(exp_dmp), rel=1e-9, abs=1e-9)
            assert api_dmn[ts] == pytest.approx(float(exp_dmn), rel=1e-9, abs=1e-9)

        # 4. Relative Strength vs VNINDEX
        res_rs = client.get(f"/api/replay/sessions/{session_id}/indicators", params={"indicator": "relative_strength", "length": 20, "benchmark": "VNINDEX"})
        assert res_rs.status_code == 200, res_rs.text
        api_rs = _value_by_timestamp(res_rs.json()["data"], "rs_vnindex_")
        assert len(api_rs) > 0
    finally:
        client.close()
        app.dependency_overrides.clear()
        db.close()
        Base.metadata.drop_all(bind=engine)
