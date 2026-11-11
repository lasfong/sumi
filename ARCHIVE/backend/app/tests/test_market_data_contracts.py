"""Unit and integration tests for provider-neutral market data port and BB contracts.

Covers:
- FR-CORE-001: Strict domain model isolation from vendor schemas and SQLite ORMs.
- FR-CORE-012: Explicit value-source precedence (ACTUAL_MATCHED_VALUE vs ESTIMATED_TP_X_VOLUME).
- FR-CORE-014: Capability tagging, method classification (OHLCV_PROXY), and rejection of unvalidated methods.
- DATA-CANDLE-001/002/003: Physical price and volume boundary validation.
- NFR-DQ-001: Missing-vs-zero semantics and non-silent value fallback.
- TEST-BB-001/002: BB horizon, request, and metric point contracts with threshold guardrails.
"""

from datetime import date, datetime, timedelta
import pytest
import pandas as pd

from app.domain.data.contracts import (
    AdjustmentType,
    CanonicalBar,
    DataIntegrityError,
    DataQuality,
    FlowMethod,
    MarketDataCapabilityManifest,
    ValueSource,
    calculate_canonical_trading_value,
    validate_candle_bounds,
)
from app.domain.data.adapters.sumi_candle_adapter import SumiCandleAdapter
from app.domain.data.adapters.fixture_adapter import FixtureMarketDataAdapter
from app.domain.data.adapters.doraemon_adapter import DoraemonMarketDataAdapter
from app.domain.bb.contracts import (
    BBHorizon,
    BBMetricPoint,
    BBSymbolRequest,
    validate_bb_threshold_semantics,
)
from app.models.candle import Candle


# ============================================================================
# 1. CanonicalBar Creation & Attributes
# ============================================================================

def test_canonical_bar_creation_and_attributes():
    """Verify CanonicalBar correctly stores attributes and computes typical price."""
    ts = datetime(2024, 1, 15, 0, 0, 0)
    bar = CanonicalBar(
        symbol="HPG",
        timestamp=ts,
        session_date=date(2024, 1, 15),
        open=25.0,
        high=27.0,
        low=24.0,
        close=26.0,
        volume=1_000_000.0,
        trading_value=25_666_666.67,
        value_source=ValueSource.ESTIMATED_TP_X_VOLUME,
        flow_method=FlowMethod.OHLCV_PROXY,
        adjustment_type=AdjustmentType.UNADJUSTED,
        quality=DataQuality.HIGH,
        source_provider="kbs_daily",
    )

    assert bar.symbol == "HPG"
    assert bar.session_date == date(2024, 1, 15)
    # Typical price: (27 + 24 + 26) / 3 = 77 / 3 = 25.666666666666668
    assert pytest.approx(bar.typical_price, rel=1e-6) == 77.0 / 3.0
    assert bar.value_source == ValueSource.ESTIMATED_TP_X_VOLUME
    assert bar.flow_method == FlowMethod.OHLCV_PROXY
    assert bar.quality == DataQuality.HIGH

    d = bar.to_dict()
    assert d["symbol"] == "HPG"
    assert d["value_source"] == "ESTIMATED_TP_X_VOLUME"
    assert d["flow_method"] == "OHLCV_PROXY"
    assert d["quality"] == "HIGH"


# ============================================================================
# 2. DATA-CANDLE-001/002/003: Price and Volume Boundary Validation
# ============================================================================

def test_price_boundary_validation():
    """Verify physical candle bounds are strictly enforced (DATA-CANDLE-001/002/003)."""
    ts = datetime(2024, 1, 15)

    # Valid bounds: OK
    is_valid, errs = validate_candle_bounds(open_price=20.0, high_price=22.0, low_price=19.0, close_price=21.0, volume=100.0)
    assert is_valid is True
    assert len(errs) == 0

    # 1. high < low
    with pytest.raises(DataIntegrityError, match="cannot be less than low price"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=20.0, high=18.0, low=19.0, close=20.0, volume=100.0
        )

    # 2. high < open
    with pytest.raises(DataIntegrityError, match="cannot be less than open price"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=25.0, high=22.0, low=20.0, close=21.0, volume=100.0
        )

    # 3. high < close
    with pytest.raises(DataIntegrityError, match="cannot be less than close price"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=20.0, high=22.0, low=19.0, close=23.0, volume=100.0
        )

    # 4. low > open
    with pytest.raises(DataIntegrityError, match="cannot be greater than open price"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=18.0, high=22.0, low=19.0, close=21.0, volume=100.0
        )

    # 5. low > close
    with pytest.raises(DataIntegrityError, match="cannot be greater than close price"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=20.0, high=22.0, low=19.0, close=18.0, volume=100.0
        )

    # 6. Negative volume
    with pytest.raises(DataIntegrityError, match="volume cannot be negative"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=20.0, high=22.0, low=19.0, close=21.0, volume=-100.0
        )

    # 7. Non-positive price
    with pytest.raises(DataIntegrityError, match="must be a positive finite number"):
        CanonicalBar(
            symbol="HPG", timestamp=ts, session_date=ts.date(),
            open=0.0, high=22.0, low=0.0, close=21.0, volume=100.0
        )

    # Explicit DataQuality.INVALID bypasses post_init exception for dirty data pipelines
    invalid_bar = CanonicalBar(
        symbol="HPG", timestamp=ts, session_date=ts.date(),
        open=25.0, high=20.0, low=18.0, close=19.0, volume=100.0,
        quality=DataQuality.INVALID,
    )
    assert invalid_bar.quality == DataQuality.INVALID


# ============================================================================
# 3. Value Source Precedence & Non-Silent Fallback
# ============================================================================

def test_value_source_precedence():
    """Verify value-source precedence rules (DATA-CANDLE-003, FR-CORE-012)."""
    # 1. Reported value is positive -> ACTUAL_MATCHED_VALUE
    val, src = calculate_canonical_trading_value(
        open_price=20.0, high_price=22.0, low_price=19.0, close_price=21.0,
        volume=1000.0, reported_value=20_500_000.0,
    )
    assert val == 20_500_000.0
    assert src == ValueSource.ACTUAL_MATCHED_VALUE

    # 2. Reported value is missing, volume > 0 -> ESTIMATED_TP_X_VOLUME
    # Typical price = (22 + 19 + 22) / 3 = 21.0 -> 21.0 * 1000 = 21,000.0
    val, src = calculate_canonical_trading_value(
        open_price=20.0, high_price=22.0, low_price=19.0, close_price=22.0,
        volume=1000.0, reported_value=None,
    )
    assert val == 21_000.0
    assert src == ValueSource.ESTIMATED_TP_X_VOLUME

    # 3. Reported value is 0 or negative, volume > 0 -> falls back to ESTIMATED_TP_X_VOLUME
    val, src = calculate_canonical_trading_value(
        open_price=20.0, high_price=22.0, low_price=19.0, close_price=22.0,
        volume=1000.0, reported_value=0.0,
    )
    assert val == 21_000.0
    assert src == ValueSource.ESTIMATED_TP_X_VOLUME

    # 4. Volume is 0 -> UNAVAILABLE with 0.0 value
    val, src = calculate_canonical_trading_value(
        open_price=20.0, high_price=20.0, low_price=20.0, close_price=20.0,
        volume=0.0, reported_value=None,
    )
    assert val == 0.0
    assert src == ValueSource.UNAVAILABLE


# ============================================================================
# 4. Missing vs. Zero Semantics (NFR-DQ-001)
# ============================================================================

def test_missing_vs_zero_semantics():
    """Verify distinct semantics for 0 volume vs None volume, and 0 value vs None value."""
    # Volume is 0 (valid traded session with 0 volume, e.g. halted/suspended)
    val_zero, src_zero = calculate_canonical_trading_value(
        open_price=10.0, high_price=10.0, low_price=10.0, close_price=10.0,
        volume=0.0, reported_value=None,
    )
    assert val_zero == 0.0
    assert src_zero == ValueSource.UNAVAILABLE

    # Volume is None (missing observation data)
    val_none, src_none = calculate_canonical_trading_value(
        open_price=10.0, high_price=10.0, low_price=10.0, close_price=10.0,
        volume=None, reported_value=None,
    )
    assert val_none is None
    assert src_none == ValueSource.UNAVAILABLE


# ============================================================================
# 5. Sumi Candle Adapter
# ============================================================================

def test_sumi_candle_adapter(db_session):
    """Verify SumiCandleAdapter maps database Candle rows to CanonicalBar sequences."""
    base_date = date(2024, 1, 1)
    candles = []
    for i in range(10):
        candles.append(Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=20.0 + i,
            high=22.0 + i,
            low=19.0 + i,
            close=21.0 + i,
            volume=500_000.0,
        ))
    db_session.add_all(candles)
    db_session.commit()

    adapter = SumiCandleAdapter(db_session)
    manifest = adapter.get_capability_manifest()
    assert manifest.provider_name == "sumi_local_candles"
    assert FlowMethod.OHLCV_PROXY in manifest.supported_flow_methods
    assert ValueSource.ESTIMATED_TP_X_VOLUME in manifest.supported_value_sources

    bars = adapter.get_canonical_bars(
        symbol="HPG",
        start_date="2024-01-02",
        end_date="2024-01-08",
    )

    # end_before includes 2024-01-08 (days 2, 3, 4, 5, 6, 7, 8 = 7 days)
    assert len(bars) == 7
    assert bars[0].symbol == "HPG"
    assert bars[0].open == 21.0
    assert bars[0].value_source == ValueSource.ESTIMATED_TP_X_VOLUME
    assert bars[0].flow_method == FlowMethod.OHLCV_PROXY
    assert bars[0].quality == DataQuality.HIGH

    # Verify as_of cutoff (days 2, 3, 4, 5 = 4 days)
    bars_as_of = adapter.get_canonical_bars(
        symbol="HPG",
        start_date="2024-01-02",
        end_date="2024-01-08",
        as_of="2024-01-05",
    )
    assert len(bars_as_of) == 4
    assert bars_as_of[-1].session_date == date(2024, 1, 5)


# ============================================================================
# 6. Fixture Market Data Adapter
# ============================================================================

def test_fixture_market_data_adapter():
    """Verify FixtureMarketDataAdapter parses in-memory dicts and filters by date and as_of."""
    adapter = FixtureMarketDataAdapter([
        {
            "symbol": "SSI",
            "timestamp": "2024-01-02T00:00:00",
            "open": 30.0, "high": 32.0, "low": 29.0, "close": 31.0,
            "volume": 200_000.0,
            "trading_value": 6_100_000.0,
            "value_source": "ACTUAL_MATCHED_VALUE",
        },
        {
            "symbol": "SSI",
            "timestamp": "2024-01-03T00:00:00",
            "open": 31.0, "high": 33.0, "low": 30.5, "close": 32.0,
            "volume": 300_000.0,
        },
    ])

    manifest = adapter.get_capability_manifest()
    assert manifest.provider_name == "fixture_adapter"

    bars = adapter.get_canonical_bars("SSI", "2024-01-01", "2024-01-10")
    assert len(bars) == 2
    assert bars[0].value_source == ValueSource.ACTUAL_MATCHED_VALUE
    assert bars[0].trading_value == 6_100_000.0
    assert bars[1].value_source == ValueSource.ESTIMATED_TP_X_VOLUME


# ============================================================================
# 7. Doraemon Market Data Adapter
# ============================================================================

def test_doraemon_market_data_adapter_parsing():
    """Verify DoraemonMarketDataAdapter correctly parses raw price payloads and handles sparse values."""
    adapter = DoraemonMarketDataAdapter()
    manifest = adapter.get_capability_manifest()
    assert manifest.provider_name == "doraemon_market_api"
    assert FlowMethod.OHLCV_PROXY in manifest.supported_flow_methods

    # Simulated response from /market/prices/FPT/history/full
    sample_payload = {
        "status": "ok",
        "data": {
            "prices": [
                {
                    "symbol": "FPT",
                    "trading_date": "2024-01-10",
                    "open_price": 95.0,
                    "high_price": 97.0,
                    "low_price": 94.0,
                    "close_price": 96.0,
                    "volume": 1_500_000.0,
                    "total_trade_value": 143_500_000.0,  # Actual value populated
                    "adjusted_close_price": 96.0,
                    "source": "kbs",
                },
                {
                    "symbol": "FPT",
                    "trading_date": "2024-01-11",
                    "open_price": 96.0,
                    "high_price": 98.0,
                    "low_price": 95.0,
                    "close_price": 97.0,
                    "volume": 2_000_000.0,
                    "total_trade_value": None,  # Sparse / missing value
                    "adjusted_close_price": None,
                    "source": "kbs",
                },
            ]
        }
    }

    bars = adapter.parse_payload(sample_payload, "FPT")
    assert len(bars) == 2

    # Row 1: Actual value populated
    assert bars[0].symbol == "FPT"
    assert bars[0].session_date == date(2024, 1, 10)
    assert bars[0].trading_value == 143_500_000.0
    assert bars[0].value_source == ValueSource.ACTUAL_MATCHED_VALUE
    assert bars[0].source_provider == "doraemon_kbs"

    # Row 2: Value missing -> falls back to ESTIMATED_TP_X_VOLUME
    # Typical price = (98 + 95 + 97) / 3 = 96.66666666666667
    # Trading value = 96.66666666666667 * 2_000_000 = 193_333_333.33333334
    assert bars[1].value_source == ValueSource.ESTIMATED_TP_X_VOLUME
    assert pytest.approx(bars[1].trading_value, rel=1e-4) == (290.0 / 3.0) * 2_000_000.0


# ============================================================================
# 8. Money Flow Blackbox (BB) Contracts & Guardrails
# ============================================================================

def test_bb_symbol_request_validation_and_incompatible_method_rejection():
    """Verify BBSymbolRequest enforces validated OHLCV_PROXY method and rejects unvalidated methods."""
    # 1. Valid request with default OHLCV_PROXY
    req = BBSymbolRequest(symbol="HPG")
    assert req.symbol == "HPG"
    assert BBHorizon.T20 in req.horizons
    assert FlowMethod.OHLCV_PROXY in req.accepted_methods

    # 2. Empty symbol raises ValueError
    with pytest.raises(ValueError, match="symbol cannot be empty"):
        BBSymbolRequest(symbol="")

    # 3. Incompatible method rejection (FR-CORE-014)
    with pytest.raises(ValueError, match="is not authorized for daily Symbol BB"):
        BBSymbolRequest(
            symbol="HPG",
            accepted_methods=[FlowMethod.CLASSIFIED_ORDER_FLOW],
        )

    with pytest.raises(ValueError, match="is not authorized for daily Symbol BB"):
        BBSymbolRequest(
            symbol="HPG",
            accepted_methods=[FlowMethod.EXECUTED_ORDER_FLOW],
        )

    with pytest.raises(ValueError, match="is not authorized for daily Symbol BB"):
        BBSymbolRequest(
            symbol="HPG",
            accepted_methods=[FlowMethod.TICK_TEST_ESTIMATE_RESEARCH_ONLY],
        )


def test_bb_metric_point_structure():
    """Verify BBMetricPoint captures complete provenance and metadata (TEST-BB-001)."""
    pt = BBMetricPoint(
        date="2024-01-15",
        horizon=BBHorizon.T20,
        bb_value=62.45,
        flow_method=FlowMethod.OHLCV_PROXY,
        value_source=ValueSource.ESTIMATED_TP_X_VOLUME,
        quality=DataQuality.HIGH,
        raw_numerator=125_000_000.0,
        raw_denominator=200_150_000.0,
        is_warmup=False,
    )

    assert pt.date == "2024-01-15"
    assert pt.horizon == BBHorizon.T20
    assert pt.horizon.lookback_bars == 20
    assert pt.bb_value == 62.45
    assert pt.flow_method == FlowMethod.OHLCV_PROXY
    assert pt.value_source == ValueSource.ESTIMATED_TP_X_VOLUME

    d = pt.to_dict()
    assert d["horizon"] == "T20"
    assert d["bb_value"] == 62.45
    assert d["flow_method"] == "OHLCV_PROXY"
    assert d["is_warmup"] is False


def test_bb_threshold_semantics_guardrail():
    """Verify guardrail prevents hardcoding 20/30/70/80 threshold semantics before measurement validation."""
    with pytest.raises(ValueError, match="cannot be hardcoded as an authoritative production signal rule"):
        validate_bb_threshold_semantics(20.0)

    with pytest.raises(ValueError, match="cannot be hardcoded as an authoritative production signal rule"):
        validate_bb_threshold_semantics(30.0)

    with pytest.raises(ValueError, match="cannot be hardcoded as an authoritative production signal rule"):
        validate_bb_threshold_semantics(70.0)

    with pytest.raises(ValueError, match="cannot be hardcoded as an authoritative production signal rule"):
        validate_bb_threshold_semantics(80.0)

    # Arbitrary reference points (e.g. midpoint 50.0) do not trigger the guardrail
    validate_bb_threshold_semantics(50.0)
