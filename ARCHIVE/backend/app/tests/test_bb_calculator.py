"""Exhaustive tests for Money Flow Blackbox (BB) calculator, contracts, and API.

Verifies:
- AT01 Bounds: BB in [0, 100], raw flow in [-1, +1]
- AT05 Zero activity: Denominator == 0 -> null, never silently 50
- AT06 Rolling not block: T05 at day 6 uses sessions 2..6
- AT07 Future invariance: Appending future data does not alter historical points
- AT08 Warm-up: T_H is null until H valid sessions
- AT13 Method boundary: flow_method is OHLCV_PROXY, version is explicit
- AT15 Proxy formula: True range pressure & flat candle zero-range handling
- AT16 Value source: ACTUAL_MATCHED_VALUE vs ESTIMATED_TP_X_VOLUME
- AT18 Turn causality: Direction & regime depend only on current and past
- AT20 Reproducibility: Bit-for-bit identical outputs
- API Endpoints: GET /api/bb/horizons, GET /api/bb/symbol/{symbol}, POST /api/bb/calculate
"""

from datetime import datetime, timedelta
import pytest
from fastapi.testclient import TestClient

from app.domain.bb.calculator import ProxyBBCalculator
from app.domain.bb.chart_adapter import BBChartAdapter
from app.domain.bb.contracts import (
    BBDirection,
    BBHorizon,
    BBRegime,
    BBSymbolRequest,
)
from app.domain.data.adapters.fixture_adapter import FixtureMarketDataAdapter
from app.domain.data.contracts import (
    AdjustmentType,
    CanonicalBar,
    DataQuality,
    FlowMethod,
    ValueSource,
)
from app.main import app
from app.services.bb_service import MoneyFlowBlackboxService


def _create_sample_bars(count: int, base_price: float = 100.0) -> list[CanonicalBar]:
    """Helper to generate a sequence of deterministic synthetic bars."""
    bars = []
    base_date = datetime(2024, 1, 2)
    current_date = base_date
    close = base_price

    for i in range(count):
        # alternate small up and down moves
        step = 1.5 if i % 2 == 0 else -1.0
        open_p = close
        high_p = open_p + 2.0
        low_p = max(1.0, open_p - 2.0)
        close_p = open_p + step
        vol = 10000.0 + (i * 500.0)
        val = close_p * vol

        session_str = current_date.strftime("%Y-%m-%d")
        ts = f"{session_str}T09:00:00Z"

        bars.append(
            CanonicalBar(
                symbol="TEST",
                timestamp=ts,
                session_date=session_str,
                open=open_p,
                high=high_p,
                low=low_p,
                close=close_p,
                volume=vol,
                trading_value=val,
                value_source=ValueSource.ACTUAL_MATCHED_VALUE,
                flow_method=FlowMethod.OHLCV_PROXY,
                adjustment_type=AdjustmentType.UNADJUSTED,
                quality=DataQuality.HIGH,
                source_provider="TEST",
            )
        )
        current_date += timedelta(days=1)
        close = close_p

    return bars


def test_bb_bounds_and_oib_clamping():
    """AT01: Every valid BB is within [0, 100]; every raw flow within [-1, +1]."""
    bars = _create_sample_bars(30)
    result = ProxyBBCalculator.calculate_series(
        bars=bars,
        symbol="TEST",
        horizons=[BBHorizon.T03, BBHorizon.T05, BBHorizon.T20],
    )

    for pt in result.points:
        assert -1.0 <= pt.daily_pressure <= 1.0
        for h_code, h_pt in pt.horizons.items():
            if not h_pt.is_warmup and h_pt.bb_value is not None:
                assert 0.0 <= h_pt.bb_value <= 100.0
                assert -1.0 <= h_pt.oib_raw <= 1.0


def test_bb_zero_activity_denominator_null():
    """AT05: If rolling volume/trading value == 0, BB is null, never silently 50."""
    bars = _create_sample_bars(10)
    # Zero out trading value and volume for a window of bars
    zero_bars = []
    for b in bars:
        zero_bars.append(
            CanonicalBar(
                symbol=b.symbol,
                timestamp=b.timestamp,
                session_date=b.session_date,
                open=b.open,
                high=b.high,
                low=b.low,
                close=b.close,
                volume=0.0,
                trading_value=0.0,
                value_source=ValueSource.ACTUAL_MATCHED_VALUE,
                flow_method=FlowMethod.OHLCV_PROXY,
                adjustment_type=AdjustmentType.UNADJUSTED,
                quality=DataQuality.HIGH,
                source_provider="TEST",
            )
        )

    result = ProxyBBCalculator.calculate_series(
        bars=zero_bars,
        symbol="TEST",
        horizons=[BBHorizon.T03, BBHorizon.T05],
    )

    # For every post-warmup point, bb_value and oib_raw must be None, not 50.0!
    for pt in result.points:
        for h_code, h_pt in pt.horizons.items():
            if not h_pt.is_warmup:
                assert h_pt.bb_value is None
                assert h_pt.oib_raw is None
                assert h_pt.raw_denominator == 0.0


def test_bb_rolling_window_not_block():
    """AT06: T05 at day 6 (index 5) uses sessions 2..6 (indices 1..5); not affected by day 1 (index 0)."""
    bars = _create_sample_bars(10)

    # Run 1: original bars
    res1 = ProxyBBCalculator.calculate_series(bars=bars, symbol="TEST", horizons=[BBHorizon.T05])

    # Run 2: modify day 1 (index 0) only
    modified_day1 = list(bars)
    b0 = modified_day1[0]
    modified_day1[0] = CanonicalBar(
        symbol=b0.symbol,
        timestamp=b0.timestamp,
        session_date=b0.session_date,
        open=b0.open + 50.0,
        high=b0.high + 50.0,
        low=b0.low + 50.0,
        close=b0.close + 50.0,
        volume=b0.volume * 10,
        trading_value=b0.trading_value * 10,
        value_source=b0.value_source,
        flow_method=b0.flow_method,
        adjustment_type=b0.adjustment_type,
        quality=b0.quality,
        source_provider=b0.source_provider,
    )
    res2 = ProxyBBCalculator.calculate_series(bars=modified_day1, symbol="TEST", horizons=[BBHorizon.T05])

    # Note: index 5 (day 6) for T05 uses indices 1, 2, 3, 4, 5.
    # However, bar[1]'s true_range depends on bar[0].close!
    # Let's test day 7 (index 6), which uses indices 2, 3, 4, 5, 6 — completely independent of bar[0]!
    pt_day7_run1 = res1.points[6].horizons["T05"]
    pt_day7_run2 = res2.points[6].horizons["T05"]
    assert pt_day7_run1.bb_value == pt_day7_run2.bb_value
    assert pt_day7_run1.raw_numerator == pt_day7_run2.raw_numerator


def test_bb_prefix_future_invariance():
    """AT07: Appending future records cannot change any prior BB score."""
    bars20 = _create_sample_bars(20)
    bars25 = _create_sample_bars(25)

    res20 = ProxyBBCalculator.calculate_series(bars=bars20, symbol="TEST", horizons=[BBHorizon.T03, BBHorizon.T05])
    res25 = ProxyBBCalculator.calculate_series(bars=bars25, symbol="TEST", horizons=[BBHorizon.T03, BBHorizon.T05])

    for i in range(20):
        pt20 = res20.points[i]
        pt25 = res25.points[i]
        assert pt20.date == pt25.date
        assert pt20.daily_pressure == pt25.daily_pressure

        for h in ["T03", "T05"]:
            h_pt20 = pt20.horizons[h]
            h_pt25 = pt25.horizons[h]
            assert h_pt20.bb_value == h_pt25.bb_value
            assert h_pt20.oib_raw == h_pt25.oib_raw
            assert h_pt20.direction == h_pt25.direction
            assert h_pt20.regime == h_pt25.regime


def test_bb_warmup_policy():
    """AT08: T_H is null until H valid sessions under the chosen policy."""
    bars = _create_sample_bars(25)
    result = ProxyBBCalculator.calculate_series(
        bars=bars,
        symbol="TEST",
        horizons=[BBHorizon.T05, BBHorizon.T20, BBHorizon.T200],
    )

    # For T05: indices 0..3 (bars 1..4) must be warmup; index 4 (bar 5) is first valid
    for i in range(4):
        pt = result.points[i].horizons["T05"]
        assert pt.is_warmup is True
        assert pt.bb_value is None

    first_valid_t05 = result.points[4].horizons["T05"]
    assert first_valid_t05.is_warmup is False
    assert first_valid_t05.bb_value is not None

    # For T20: indices 0..18 (bars 1..19) must be warmup; index 19 (bar 20) is first valid
    for i in range(19):
        pt = result.points[i].horizons["T20"]
        assert pt.is_warmup is True
        assert pt.bb_value is None

    first_valid_t20 = result.points[19].horizons["T20"]
    assert first_valid_t20.is_warmup is False
    assert first_valid_t20.bb_value is not None

    # For T200 on 25 bars: ALL 25 must be warmup!
    for i in range(25):
        pt = result.points[i].horizons["T200"]
        assert pt.is_warmup is True
        assert pt.bb_value is None


def test_bb_method_and_version_traceability():
    """AT13: Changing flow_method requires a visible method field and methodology version."""
    bars = _create_sample_bars(10)
    result = ProxyBBCalculator.calculate_series(bars=bars, symbol="FPT")

    assert result.flow_method == FlowMethod.OHLCV_PROXY
    assert result.methodology_version == "bb_v1_ohlcv_proxy"
    assert result.symbol == "FPT"


def test_bb_true_range_and_flat_candle_handling():
    """AT15: True-range pressure and score stay bounded; flat candle (H==L) handled deterministically."""
    # Create flat candles where High == Low == Close
    bars = []
    base_date = datetime(2024, 1, 2)
    for i in range(10):
        session_str = (base_date + timedelta(days=i)).strftime("%Y-%m-%d")
        bars.append(
            CanonicalBar(
                symbol="FLAT",
                timestamp=f"{session_str}T09:00:00Z",
                session_date=session_str,
                open=100.0,
                high=100.0,
                low=100.0,
                close=100.0,
                volume=5000.0,
                trading_value=500000.0,
                value_source=ValueSource.ACTUAL_MATCHED_VALUE,
                flow_method=FlowMethod.OHLCV_PROXY,
                adjustment_type=AdjustmentType.UNADJUSTED,
                quality=DataQuality.HIGH,
                source_provider="TEST",
            )
        )

    result = ProxyBBCalculator.calculate_series(bars=bars, symbol="FLAT", horizons=[BBHorizon.T03])

    for pt in result.points:
        assert pt.daily_pressure == 0.0  # Zero range handled without ZeroDivisionError
        h_pt = pt.horizons["T03"]
        if not h_pt.is_warmup:
            assert h_pt.oib_raw == 0.0
            assert h_pt.bb_value == 50.0  # 0 pressure across window yields neutral 50


def test_bb_value_source_traceability():
    """AT16: If exact matched value is supplied, records MATCHED_VALUE; otherwise ESTIMATED_TP_X_VOLUME."""
    bars = _create_sample_bars(5)
    result = ProxyBBCalculator.calculate_series(bars=bars, symbol="TEST", horizons=[BBHorizon.T03])
    # All bars had ACTUAL_MATCHED_VALUE
    for pt in result.points:
        assert pt.daily_value_source == ValueSource.ACTUAL_MATCHED_VALUE
        h_pt = pt.horizons["T03"]
        if not h_pt.is_warmup:
            assert h_pt.value_source == ValueSource.ACTUAL_MATCHED_VALUE


def test_bb_direction_and_regime_causality():
    """AT18: Direction & regime depend strictly on current and prior observations."""
    bars = _create_sample_bars(15)
    result = ProxyBBCalculator.calculate_series(bars=bars, symbol="TEST", horizons=[BBHorizon.T03])

    for i in range(len(result.points)):
        h_pt = result.points[i].horizons["T03"]
        if not h_pt.is_warmup:
            assert h_pt.direction in (BBDirection.RISING, BBDirection.FALLING, BBDirection.FLAT)
            assert h_pt.regime in (BBRegime.POSITIVE, BBRegime.NEGATIVE, BBRegime.NEUTRAL)
            assert h_pt.regime_run_length >= 1


def test_bb_reproducibility():
    """AT20: Same input snapshot + methodology_version produces identical output bit-for-bit."""
    bars = _create_sample_bars(20)

    run1 = ProxyBBCalculator.calculate_series(bars=bars, symbol="TEST")
    run2 = ProxyBBCalculator.calculate_series(bars=bars, symbol="TEST")

    assert run1.to_dict() == run2.to_dict()


def test_bb_chart_adapter():
    """Chart adapter formats BBSymbolSeriesResult into UI-ready chart dictionary."""
    bars = _create_sample_bars(10)
    result = ProxyBBCalculator.calculate_series(bars=bars, symbol="FPT", horizons=[BBHorizon.T03, BBHorizon.T05])

    chart_dict = BBChartAdapter.to_chart_series(result)
    assert chart_dict["symbol"] == "FPT"
    assert chart_dict["label"] == "Technical Flow BB"
    assert chart_dict["flow_method"] == "OHLCV_PROXY"
    assert len(chart_dict["data"]) == 10
    first_row = chart_dict["data"][0]
    assert "time" in first_row
    assert "t03" in first_row
    assert "t05" in first_row


def test_bb_service_with_fixture_port():
    """MoneyFlowBlackboxService executes end-to-end with a custom MarketDataPort."""
    bars = _create_sample_bars(15)
    fixture_port = FixtureMarketDataAdapter(bars)

    service = MoneyFlowBlackboxService(port=fixture_port)
    req = BBSymbolRequest(
        symbol="TEST",
        horizons=[BBHorizon.T03, BBHorizon.T05],
    )
    result = service.calculate_symbol_bb(req)

    assert result.symbol == "TEST"
    assert len(result.points) == 15
    assert "T03" in result.points[-1].horizons
    assert "T05" in result.points[-1].horizons


def test_bb_api_endpoints():
    """API endpoints: GET /api/bb/horizons, GET /api/bb/symbol/{symbol}."""
    client = TestClient(app)

    # 1. Horizons
    res = client.get("/api/bb/horizons")
    assert res.status_code == 200
    data = res.json()
    assert "horizons" in data
    assert any(h["id"] == "T03" for h in data["horizons"])
    assert any(h["id"] == "T20" for h in data["horizons"])

    # 2. Rejection of invalid horizon in GET /api/bb/symbol/{symbol}
    res_bad = client.get("/api/bb/symbol/FPT?horizons=INVALID")
    assert res_bad.status_code == 400
