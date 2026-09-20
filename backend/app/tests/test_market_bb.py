"""Comprehensive test suite for Market Blackbox, Flow Breadth, and Publication Gate.

Verifies:
- BB-MKT-001 / AT09: Aggregate-before-ratio identity.
- BB-MKT-002 / AT10: Constructed mathematical proof that mean(symbol BB) != Market BB.
- BB-MKT-003 / AT11: Universe Point-in-Time (PIT) boundary enforcement.
- BB-MKT-004 / AT17: Flow Breadth consistency (counts and values sum to 1.0).
- AT12: Missing data drops coverage rather than injecting zero values.
- AT05 / AT08: Zero activity and warmup handling.
- BB-MKT-005: Publication Gate enforcement (CANDIDATE / retrospective / degraded coverage).
- API contract endpoints for Market BB.
"""

from datetime import date
from typing import List
import pytest
from fastapi.testclient import TestClient

from app.domain.bb.contracts import (
    BBHorizon,
    BBDirection,
    BBRegime,
    MarketPublicationStatus,
)
from app.domain.bb.market_aggregator import MarketBBAggregator
from app.domain.data.contracts import CanonicalBar, DataQuality, FlowMethod, ValueSource
from app.domain.universe.models import (
    CoverageStatus,
    MemberStatus,
    UniverseDefinition,
    UniverseMember,
    UniverseMode,
    UniverseStatus,
)
from app.main import app


def _make_bar(
    symbol: str,
    session_date: str,
    open_: float,
    high: float,
    low: float,
    close: float,
    volume: float,
    trading_value: float,
) -> CanonicalBar:
    return CanonicalBar(
        symbol=symbol,
        session_date=session_date,
        timestamp=f"{session_date}T15:00:00Z",
        open=open_,
        high=high,
        low=low,
        close=close,
        volume=volume,
        trading_value=trading_value,
        value_source=ValueSource.ACTUAL_MATCHED_VALUE,
        quality=DataQuality.HIGH,
    )


def _make_sample_universe(
    universe_id: str = "TEST_MKT_UNI",
    status: UniverseStatus = UniverseStatus.APPROVED,
    default_mode: UniverseMode = UniverseMode.POINT_IN_TIME,
    members: List[UniverseMember] = None,
) -> UniverseDefinition:
    if members is None:
        members = [
            UniverseMember(symbol="SYM_A", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
            UniverseMember(symbol="SYM_B", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
        ]
    return UniverseDefinition(
        universe_id=universe_id,
        version="v1.0",
        name="Test Market Universe",
        status=status,
        default_mode=default_mode,
        effective_from=date(2024, 1, 1),
        members=members,
    )


def test_at09_aggregate_identity():
    """AT09: Market BB from aggregated Buy/Sell equals the ratio calculated from the same aggregated values."""
    universe = _make_sample_universe()

    # 3 sessions of bars for SYM_A and SYM_B
    # Day 1:
    # SYM_A: high=105, low=95, close=105, val=100. P = (210 - 200)/10 = 1.0 -> Buy=100, Sell=0
    # SYM_B: high=55, low=45, close=45, val=100. P = (90 - 100)/10 = -1.0 -> Buy=0, Sell=100
    # MarketBuy = 100, MarketSell = 100, Total = 200. BB_T03 warmup.
    # Day 2:
    # SYM_A: high=110, low=100, close=110, val=200. P = 1.0 -> Buy=200, Sell=0
    # SYM_B: high=50, low=40, close=45, val=100. P = (90 - 90)/10 = 0.0 -> Buy=50, Sell=50
    # MarketBuy = 250, MarketSell = 50, Total = 300. BB_T03 warmup.
    # Day 3:
    # SYM_A: high=115, low=105, close=110, val=100. P = 0.0 -> Buy=50, Sell=50
    # SYM_B: high=50, low=40, close=50, val=200. P = 1.0 -> Buy=200, Sell=0
    # MarketBuy = 250, MarketSell = 50, Total = 300.
    # Rolling T03 sum (Day 1..3):
    # TotalBuy = 100 + 250 + 250 = 600
    # TotalSell = 100 + 50 + 50 = 200
    # TotalVal = 600 + 200 = 800
    # Expected MarketBB_T03 = 100 * (600 / 800) = 75.0

    bars_a = [
        _make_bar("SYM_A", "2024-01-02", 100, 105, 95, 105, 1.0, 100.0),
        _make_bar("SYM_A", "2024-01-03", 105, 110, 100, 110, 2.0, 200.0),
        _make_bar("SYM_A", "2024-01-04", 110, 115, 105, 110, 1.0, 100.0),
    ]
    bars_b = [
        _make_bar("SYM_B", "2024-01-02", 50, 55, 45, 45, 2.0, 100.0),
        _make_bar("SYM_B", "2024-01-03", 45, 50, 40, 45, 2.0, 100.0),
        _make_bar("SYM_B", "2024-01-04", 45, 50, 40, 50, 4.0, 200.0),
    ]

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={"SYM_A": bars_a, "SYM_B": bars_b},
        horizons=[BBHorizon.T03],
    )

    assert len(res.points) == 3
    day3_pt = res.points[2]
    h_pt = day3_pt.horizons["T03"]

    assert h_pt.is_warmup is False
    assert h_pt.market_buy_value == 600.0
    assert h_pt.market_sell_value == 200.0
    assert h_pt.market_total_value == 800.0
    # Direct formula check (AT09 identity)
    expected_ratio = 100.0 * (600.0 / 800.0)
    assert h_pt.market_bb_value == pytest.approx(expected_ratio, rel=1e-4)
    assert h_pt.market_bb_value == 75.0
    assert h_pt.regime == BBRegime.POSITIVE


def test_at10_no_score_averaging_counterexample():
    """AT10: A deliberately constructed dataset demonstrates that mean(symbol BB) != core Market BB.
    
    Verifies that Market BB is trading-value weighted by construction and strictly differs from
    unweighted score averaging.
    """
    universe = _make_sample_universe()

    # Construct two symbols with 3 identical days:
    # Symbol A: Large cap with heavy trading value = 100M, strong buying pressure (P = 0.8) -> BB = 90.0
    # Buy = 90M, Sell = 10M
    # Symbol B: Small cap with small trading value = 10M, strong selling pressure (P = -0.8) -> BB = 10.0
    # Buy = 1M, Sell = 9M
    #
    # Unweighted Average of scores:
    # mean(BB_A, BB_B) = (90.0 + 10.0) / 2 = 50.0 (Falsely indicates neutral money flow!)
    #
    # Value-weighted Market Aggregation:
    # MarketBuy = 90M + 1M = 91M
    # MarketSell = 10M + 9M = 19M
    # TotalMarketValue = 110M
    # True MarketBB = 100 * (91 / 110) = 82.72727... (Reveals strong net institutional buying power!)

    bars_a = [
        _make_bar("SYM_A", "2024-01-02", 100, 110, 90, 108, 1000, 100_000_000.0),
        _make_bar("SYM_A", "2024-01-03", 108, 118, 98, 116, 1000, 100_000_000.0),
        _make_bar("SYM_A", "2024-01-04", 116, 126, 106, 124, 1000, 100_000_000.0),
    ]
    bars_b = [
        _make_bar("SYM_B", "2024-01-02", 50, 60, 40, 42, 100, 10_000_000.0),
        _make_bar("SYM_B", "2024-01-03", 42, 52, 32, 34, 100, 10_000_000.0),
        _make_bar("SYM_B", "2024-01-04", 34, 44, 24, 26, 100, 10_000_000.0),
    ]

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={"SYM_A": bars_a, "SYM_B": bars_b},
        horizons=[BBHorizon.T03],
    )

    day3 = res.points[2]
    market_bb = day3.horizons["T03"].market_bb_value
    assert market_bb is not None

    # Unweighted average of symbol scores
    mean_symbol_score = 50.0

    # Mathematical divergence proof
    assert abs(market_bb - mean_symbol_score) > 30.0
    assert market_bb == pytest.approx(82.73, abs=0.05)


def test_at11_universe_point_in_time_boundary():
    """AT11: Universe PIT: A delisted symbol contributes before delisting and not after;
    a newly listed symbol appears only from eligibility date.
    """
    # Universe with 3 members:
    # PERM: active all dates
    # DELIST: active 2024-01-01 to 2024-01-02, delisted on 2024-01-03
    # NEWLIST: effective from 2024-01-03
    members = [
        UniverseMember(symbol="PERM", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
        UniverseMember(symbol="DELIST", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1), effective_to=date(2024, 1, 2)),
        UniverseMember(symbol="NEWLIST", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 3)),
    ]
    universe = UniverseDefinition(
        universe_id="PIT_MKT_TEST",
        version="v1",
        name="PIT Test Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=members,
    )

    # Day 1 (2024-01-02): PERM (100) + DELIST (100) = 200 total value. NEWLIST has bar but is not yet listed!
    # Day 2 (2024-01-03): PERM (100) + NEWLIST (100) = 200 total value. DELIST has bar but is now delisted!
    bars_perm = [
        _make_bar("PERM", "2024-01-02", 10, 12, 8, 10, 10, 100.0),
        _make_bar("PERM", "2024-01-03", 10, 12, 8, 10, 10, 100.0),
    ]
    bars_delist = [
        _make_bar("DELIST", "2024-01-02", 20, 22, 18, 20, 10, 100.0),
        _make_bar("DELIST", "2024-01-03", 20, 22, 18, 20, 10, 1000.0),  # Should be excluded!
    ]
    bars_newlist = [
        _make_bar("NEWLIST", "2024-01-02", 30, 32, 28, 30, 10, 5000.0),  # Should be excluded!
        _make_bar("NEWLIST", "2024-01-03", 30, 32, 28, 30, 10, 100.0),
    ]

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={
            "PERM": bars_perm,
            "DELIST": bars_delist,
            "NEWLIST": bars_newlist,
        },
        mode=UniverseMode.POINT_IN_TIME,
        horizons=[BBHorizon.T03],
    )

    assert len(res.points) == 2

    # Session 1: PERM (100) + DELIST (100) = 200
    pt1 = res.points[0]
    assert pt1.reporting_symbols_count == 2
    assert pt1.total_target_count == 2
    assert pt1.horizons["T03"].market_total_value == 200.0

    # Session 2: PERM (100) + NEWLIST (100) = 200 (DELIST excluded despite having bars)
    pt2 = res.points[1]
    assert pt2.reporting_symbols_count == 2
    assert pt2.total_target_count == 2
    assert pt2.horizons["T03"].market_total_value == 400.0  # Cumulative 2 sessions = 200 + 200


def test_at12_missing_symbol_does_not_zero():
    """AT12: Missing vendor record triggers DQ/missing state, not zero buy/sell."""
    universe = _make_sample_universe()

    # SYM_A has bars for both days, SYM_B is missing on day 2
    bars_a = [
        _make_bar("SYM_A", "2024-01-02", 100, 105, 95, 100, 1.0, 100.0),
        _make_bar("SYM_A", "2024-01-03", 100, 105, 95, 100, 1.0, 100.0),
    ]
    bars_b = [
        _make_bar("SYM_B", "2024-01-02", 50, 55, 45, 50, 1.0, 100.0),
    ]

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={"SYM_A": bars_a, "SYM_B": bars_b},
        horizons=[BBHorizon.T03],
    )

    day2 = res.points[1]
    assert day2.reporting_symbols_count == 1
    assert day2.total_target_count == 2
    assert day2.symbol_coverage == 0.50
    assert day2.coverage_status == CoverageStatus.DEGRADED.value


def test_at17_breadth_consistency_and_sums():
    """AT17: Breadth consistency: Positive + neutral + negative count breadth sums to 1.0;
    value breadth likewise.
    """
    # 3 symbols with varied scores:
    # SYM_POS: bullish (BB ~ 90)
    # SYM_NEU: neutral (BB ~ 50)
    # SYM_NEG: bearish (BB ~ 10)
    members = [
        UniverseMember(symbol="SYM_POS", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
        UniverseMember(symbol="SYM_NEU", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
        UniverseMember(symbol="SYM_NEG", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1)),
    ]
    universe = UniverseDefinition(
        universe_id="BREADTH_UNI",
        version="v1",
        name="Breadth Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=members,
    )

    def _generate_bars(sym: str, p: float, val: float):
        # 3 sessions
        bars = []
        for d in ["2024-01-02", "2024-01-03", "2024-01-04"]:
            close = 100.0 + p * 10.0
            bars.append(_make_bar(sym, d, 100.0, 110.0, 90.0, close, 100.0, val))
        return bars

    bars_pos = _generate_bars("SYM_POS", 0.8, 100.0)
    bars_neu = _generate_bars("SYM_NEU", 0.0, 200.0)
    bars_neg = _generate_bars("SYM_NEG", -0.8, 300.0)

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={"SYM_POS": bars_pos, "SYM_NEU": bars_neu, "SYM_NEG": bars_neg},
        horizons=[BBHorizon.T03],
        breadth_buffer=2.5,
    )

    day3 = res.points[2]
    breadth = day3.breadth["T03"]

    assert breadth.positive_count == 1
    assert breadth.neutral_count == 1
    assert breadth.negative_count == 1
    assert breadth.total_count == 3

    # Invariant: count ratios sum to 1.0 within 1e-5
    count_sum = (
        breadth.positive_count_ratio + breadth.neutral_count_ratio + breadth.negative_count_ratio
    )
    assert count_sum == pytest.approx(1.0, abs=1e-5)

    # Invariant: value ratios sum to 1.0 within 1e-5
    val_sum = (
        breadth.positive_value_ratio + breadth.neutral_value_ratio + breadth.negative_value_ratio
    )
    assert val_sum == pytest.approx(1.0, abs=1e-5)


def test_at05_zero_activity_and_at08_warmup():
    """AT05 & AT08: Zero activity yields null score, never 50; warmup yields null."""
    universe = _make_sample_universe()

    # Bars with 0 trading value
    bars_zero = [
        _make_bar("SYM_A", "2024-01-02", 100, 100, 100, 100, 0.0, 0.0),
        _make_bar("SYM_A", "2024-01-03", 100, 100, 100, 100, 0.0, 0.0),
        _make_bar("SYM_A", "2024-01-04", 100, 100, 100, 100, 0.0, 0.0),
    ]

    res = MarketBBAggregator.calculate_market_bb(
        universe=universe,
        symbol_bars_map={"SYM_A": bars_zero},
        horizons=[BBHorizon.T03],
    )

    # Warmup day 1 (AT08)
    assert res.points[0].horizons["T03"].is_warmup is True
    assert res.points[0].horizons["T03"].market_bb_value is None

    # Day 3 has lookback 3, but 0 total activity (AT05)
    assert res.points[2].horizons["T03"].is_warmup is False
    assert res.points[2].horizons["T03"].market_bb_value is None


def test_publication_gate_rules():
    """BB-MKT-005: Publication Gate prevents canonical publishing when conditions are not met."""
    # 1. Candidate universe cannot be CANONICAL_PUBLISHED
    candidate_uni = _make_sample_universe(status=UniverseStatus.CANDIDATE)
    bars = [
        _make_bar("SYM_A", "2024-01-02", 100, 105, 95, 105, 1.0, 100.0),
        _make_bar("SYM_B", "2024-01-02", 50, 55, 45, 50, 1.0, 100.0),
    ]
    res_cand = MarketBBAggregator.calculate_market_bb(
        universe=candidate_uni,
        symbol_bars_map={"SYM_A": bars[:1], "SYM_B": bars[1:]},
    )
    assert res_cand.publication_status == MarketPublicationStatus.RESEARCH_RETROSPECTIVE
    assert any("CANDIDATE" in r for r in res_cand.gate_reasons)

    # 2. Retrospective mode forces RESEARCH_RETROSPECTIVE and survivor bias caveat
    approved_uni = _make_sample_universe(status=UniverseStatus.APPROVED)
    res_retro = MarketBBAggregator.calculate_market_bb(
        universe=approved_uni,
        symbol_bars_map={"SYM_A": bars[:1], "SYM_B": bars[1:]},
        mode=UniverseMode.RETROSPECTIVE_FIXED,
    )
    assert res_retro.publication_status == MarketPublicationStatus.RESEARCH_RETROSPECTIVE
    assert res_retro.survivor_bias_caveat is not None
    assert any("RETROSPECTIVE_FIXED" in r for r in res_retro.gate_reasons)

    # 3. Failed coverage (<50%) yields UNAVAILABLE_DEGRADED
    # Universe has 4 members, but only 1 has data (25% coverage)
    members_4 = [
        UniverseMember(symbol=f"S_{i}", exchange="HOSE", status=MemberStatus.ACTIVE, effective_from=date(2024, 1, 1))
        for i in range(4)
    ]
    uni_4 = UniverseDefinition(
        universe_id="UNI_4",
        version="v1",
        name="4 Member Universe",
        status=UniverseStatus.APPROVED,
        default_mode=UniverseMode.POINT_IN_TIME,
        effective_from=date(2024, 1, 1),
        members=members_4,
    )
    res_failed_cov = MarketBBAggregator.calculate_market_bb(
        universe=uni_4,
        symbol_bars_map={"S_0": [_make_bar("S_0", "2024-01-02", 10, 12, 8, 10, 1, 10)]},
    )
    assert res_failed_cov.publication_status == MarketPublicationStatus.UNAVAILABLE_DEGRADED
    assert any("Coverage failed" in r for r in res_failed_cov.gate_reasons)

    # 4. Approved PIT universe with 100% coverage succeeds with CANONICAL_PUBLISHED
    res_canon = MarketBBAggregator.calculate_market_bb(
        universe=approved_uni,
        symbol_bars_map={"SYM_A": bars[:1], "SYM_B": bars[1:]},
        mode=UniverseMode.POINT_IN_TIME,
    )
    assert res_canon.publication_status == MarketPublicationStatus.CANONICAL_PUBLISHED
    assert len(res_canon.gate_reasons) == 0


def test_market_bb_api_endpoints(client):
    """Validate FastAPI endpoints for Market BB."""

    # Test GET /api/bb/market/{universe_id} with nonexistent universe
    resp = client.get("/api/bb/market/NONEXISTENT_UNI")
    assert resp.status_code == 404

    # Test GET /api/bb/market/{universe_id} with TEST-PIT fixture universe
    resp_pit = client.get("/api/bb/market/TEST-PIT")
    assert resp_pit.status_code == 200
    data = resp_pit.json()
    assert data["universe_id"] == "TEST-PIT"
    assert data["flow_method"] == "OHLCV_PROXY"
    assert "publication_status" in data
    assert "points" in data

    # Test POST /api/bb/market/calculate
    post_resp = client.post(
        "/api/bb/market/calculate",
        json={"universe_id": "TEST-PIT", "horizons": ["T03", "T05"]},
    )
    assert post_resp.status_code == 200
    post_data = post_resp.json()
    assert post_data["universe_id"] == "TEST-PIT"
    assert "publication_status" in post_data
